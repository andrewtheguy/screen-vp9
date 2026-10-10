//! The threads an encoder codes on, made once for its life and parked
//! between frames rather than spawned for each phase of one: a frame's
//! phases are short, and a thread spawned and joined costs as much as some
//! of them. A phase hands the threads one closure, which takes work from a
//! queue the phase keeps until there is none, and returns once every
//! thread that took the closure up is done with it, so the closure may
//! borrow what the phase has on its stack.
//!
//! Waking a parked thread costs more than a small frame's phase does, so
//! nothing waits on one. The caller runs the closure itself and is done
//! when the work is: a worker that wakes after that finds the phase gone
//! and has missed it. And a worker that has finished a phase spins for
//! `SPIN` before it parks, a frame's phases following one another within
//! microseconds, so that it is there for the next.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How long a thread spins for what it waits on before it parks: longer
/// than what a frame does between two phases, and far shorter than a frame.
const SPIN: Duration = Duration::from_micros(100);
/// How many times a spinning thread looks before it reads the clock again.
const LOOKS: u32 = 32;

/// A phase's closure, which the workers are handed by address: what it
/// borrows outlives `Pool::run`, which hands it out and waits.
#[derive(Clone, Copy)]
struct Job {
    f: *const (dyn Fn(usize) + Sync),
    /// How many threads it is for, the caller's thread the first.
    count: usize,
}

// SAFETY: the closure is `Sync`, and the address is used only while
// `Pool::run` borrows the closure, which it does until every worker that
// took the job has let go of it.
unsafe impl Send for Job {}

struct State {
    /// The job posted last, until its caller is done with it, and how many
    /// jobs have been posted: a worker that has seen this many has seen
    /// this one.
    job: Option<Job>,
    posted: u64,
    /// How many workers took the posted job and have it still to finish.
    running: usize,
    /// What a worker's run of the job panicked with, for the caller.
    panic: Option<Box<dyn Any + Send>>,
    stop: bool,
}

struct Shared {
    state: Mutex<State>,
    /// `State::posted` and `State::running` as they were last set, for a
    /// thread that spins to read without the lock.
    posted: AtomicU64,
    running: AtomicUsize,
    /// Workers wait on this for a job; the caller on `done` for its end.
    wake: Condvar,
    done: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Spin until `done` or for `SPIN`, whichever is first.
    fn spin(done: impl Fn() -> bool) {
        let start = Instant::now();
        loop {
            for _ in 0..LOOKS {
                if done() {
                    return;
                }
                std::hint::spin_loop();
            }
            if start.elapsed() >= SPIN {
                return;
            }
        }
    }
}

/// `threads` threads, this one among them, for the encoder's life.
pub(super) struct Pool {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl Pool {
    pub(super) fn new(threads: usize) -> Self {
        let state = State { job: None, posted: 0, running: 0, panic: None, stop: false };
        let shared = Arc::new(Shared { state: Mutex::new(state), posted: AtomicU64::new(0), running: AtomicUsize::new(0), wake: Condvar::new(), done: Condvar::new() });
        let workers = (1..threads.max(1))
            .map(|index| {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || Self::work(&shared, index))
            })
            .collect();
        Self { shared, workers }
    }

    /// How many threads there are, this one among them.
    pub(super) fn threads(&self) -> usize {
        self.workers.len() + 1
    }

    /// Run `f` on up to `count` threads at once (as many as there are, at
    /// most): on this one as `f(0)`, and as `f(index)` on each worker that
    /// takes it up before `f(0)` returns, with its own index from 1. `f`
    /// is to take work from a queue until there is none, so that `f(0)`
    /// returns when the work is done, whoever did it. Returns once every
    /// thread that took `f` up has returned from it. A panic in any of
    /// them is raised again here once the rest have returned.
    pub(super) fn run(&self, count: usize, f: impl Fn(usize) + Sync) {
        let count = count.clamp(1, self.threads());
        if count == 1 {
            f(0);
            return;
        }
        let f: &(dyn Fn(usize) + Sync + '_) = &f;
        // SAFETY: the pointer's lifetime is erased only to be kept in the
        // shared state, which hands it to workers for as long as this call
        // borrows `f`: a worker takes it under the lock while it is
        // posted, and the call takes it down and then returns once every
        // worker that took it is done.
        let erased: *const (dyn Fn(usize) + Sync + 'static) = unsafe { std::mem::transmute(std::ptr::from_ref(f)) };
        {
            let mut state = self.shared.lock();
            state.job = Some(Job { f: erased, count });
            state.posted += 1;
            self.shared.posted.store(state.posted, Ordering::Release);
            self.shared.wake.notify_all();
        }
        let mine = catch_unwind(AssertUnwindSafe(|| f(0)));
        // No worker takes it up from here on.
        self.shared.lock().job = None;
        Shared::spin(|| self.shared.running.load(Ordering::Acquire) == 0);
        let mut state = self.shared.lock();
        while state.running > 0 {
            state = self.shared.done.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        let theirs = state.panic.take();
        drop(state);
        if let Some(panic) = theirs.or(mine.err()) {
            resume_unwind(panic);
        }
    }

    /// A worker: each job posted that is for its index and still up when
    /// it looks, run once, and between them spinning a moment and then
    /// parked.
    fn work(shared: &Shared, index: usize) {
        let mut seen = 0;
        let mut state = shared.lock();
        loop {
            if state.stop {
                return;
            }
            if state.posted == seen {
                // The next phase's job is likely on its way: spin for it,
                // and park only if it is still not there under the lock.
                drop(state);
                Shared::spin(|| shared.posted.load(Ordering::Acquire) != seen);
                state = shared.lock();
                if state.posted == seen && !state.stop {
                    state = shared.wake.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                continue;
            }
            seen = state.posted;
            let Some(job) = state.job.filter(|job| index < job.count) else { continue };
            state.running += 1;
            shared.running.store(state.running, Ordering::Release);
            drop(state);
            // SAFETY: `Pool::run` keeps the closure alive until `running`
            // is 0 with the job taken down, and this worker took the job
            // while it was up and counts in `running` until it says so
            // below.
            let f = unsafe { &*job.f };
            let outcome = catch_unwind(AssertUnwindSafe(|| f(index)));
            state = shared.lock();
            if let Err(panic) = outcome {
                state.panic.get_or_insert(panic);
            }
            state.running -= 1;
            shared.running.store(state.running, Ordering::Release);
            if state.running == 0 {
                shared.done.notify_one();
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.lock().stop = true;
        self.shared.wake.notify_all();
        for worker in self.workers.drain(..) {
            // A worker that panicked outside a job has nothing to say.
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Count this thread into `there` and hold it until `count` are: a
    /// closure that waits for every thread it is for, however long a
    /// loaded machine takes to wake them, where the encoder's would be done
    /// without them.
    fn gather(there: &AtomicUsize, count: usize) {
        there.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        while there.load(Ordering::Relaxed) < count {
            assert!(started.elapsed() < Duration::from_secs(60), "{count} threads take the closure up");
            std::thread::yield_now();
        }
    }

    #[test]
    fn every_thread_there_in_time_runs_the_closure_with_its_own_index_and_the_caller_waits() {
        let pool = Pool::new(4);
        assert_eq!(pool.threads(), 4);
        for count in [1, 2, 4, 9] {
            let ran = Mutex::new(Vec::new());
            let sum = AtomicUsize::new(0);
            let there = AtomicUsize::new(0);
            let n = count.min(4);
            pool.run(count, |i| {
                gather(&there, n);
                sum.fetch_add(i, Ordering::Relaxed);
                ran.lock().unwrap().push(i);
            });
            let mut ran = ran.into_inner().unwrap();
            ran.sort_unstable();
            assert_eq!(ran, (0..n).collect::<Vec<_>>(), "{count} asked for");
            assert_eq!(sum.into_inner(), n * (n - 1) / 2);
        }
    }

    #[test]
    fn a_borrowed_stack_is_written_and_read_back() {
        let pool = Pool::new(3);
        let mut out = [0u32; 3];
        let slots: Vec<Mutex<&mut u32>> = out.iter_mut().map(Mutex::new).collect();
        let there = AtomicUsize::new(0);
        pool.run(3, |i| {
            gather(&there, 3);
            **slots[i].lock().unwrap() = i as u32 * 10 + 1;
        });
        drop(slots);
        assert_eq!(out, [1, 11, 21]);
    }

    #[test]
    fn a_panic_on_any_thread_reaches_the_caller_after_the_rest_are_done() {
        let pool = Pool::new(3);
        for culprit in 0..3 {
            let finished = AtomicUsize::new(0);
            let there = AtomicUsize::new(0);
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                pool.run(3, |i| {
                    gather(&there, 3);
                    if i == culprit {
                        panic!("thread {i}");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    finished.fetch_add(1, Ordering::Relaxed);
                })
            }));
            let panic = outcome.expect_err("a panic");
            assert_eq!(panic.downcast_ref::<String>().map(String::as_str), Some(format!("thread {culprit}").as_str()));
            assert_eq!(finished.load(Ordering::Relaxed), 2, "the others ran to the end");
            // The pool is whole after it.
            let count = AtomicUsize::new(0);
            pool.run(3, |_| gather(&count, 3));
            assert_eq!(count.into_inner(), 3);
        }
    }

    #[test]
    fn work_the_caller_finishes_alone_waits_on_no_one() {
        let pool = Pool::new(4);
        // Parked workers, and a queue the caller empties at once: every
        // item is taken once, by whoever was there.
        std::thread::sleep(std::time::Duration::from_millis(20));
        for _ in 0..200 {
            let next = AtomicUsize::new(0);
            let taken = AtomicUsize::new(0);
            pool.run(4, |_| {
                while next.fetch_add(1, Ordering::Relaxed) < 8 {
                    taken.fetch_add(1, Ordering::Relaxed);
                }
            });
            assert_eq!(taken.into_inner(), 8);
        }
    }

    #[test]
    fn one_thread_runs_in_place_and_dropping_joins_the_rest() {
        let pool = Pool::new(1);
        let n = AtomicUsize::new(0);
        pool.run(5, |i| {
            assert_eq!(i, 0);
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.into_inner(), 1);
        let pool = Pool::new(6);
        let started = std::time::Instant::now();
        drop(pool);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }
}
