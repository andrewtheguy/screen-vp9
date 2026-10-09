//! The threads an encoder codes on, made once for its life and parked
//! between the phases of a frame rather than spawned for each: a frame's
//! phases are short, and a thread spawned and joined costs as much as some
//! of them. A phase hands every thread the same closure with its own index,
//! as `std::thread::scope` would, and returns once every thread is done
//! with it, so the closure may borrow what the phase has on its stack.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

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
    /// The job posted last, and how many jobs have been posted: a worker
    /// that has seen this many has seen this one.
    job: Option<Job>,
    posted: u64,
    /// How many workers have the posted job still to finish.
    left: usize,
    /// What a worker's run of the job panicked with, for the caller.
    panic: Option<Box<dyn Any + Send>>,
    stop: bool,
}

struct Shared {
    state: Mutex<State>,
    /// Workers wait on this for a job; the caller on `done` for its end.
    wake: Condvar,
    done: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// `threads` threads, this one among them, for the encoder's life.
pub(super) struct Pool {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl Pool {
    pub(super) fn new(threads: usize) -> Self {
        let shared = Arc::new(Shared { state: Mutex::new(State { job: None, posted: 0, left: 0, panic: None, stop: false }), wake: Condvar::new(), done: Condvar::new() });
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

    /// Run `f` on `count` threads at once (as many as there are, at most),
    /// this one as `f(0)` and each worker as `f(index)` with its own index
    /// from 1, and return once every one has returned. A panic in any of
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
        // borrows `f`: the call returns once every one of them is done.
        let erased: *const (dyn Fn(usize) + Sync + 'static) = unsafe { std::mem::transmute(std::ptr::from_ref(f)) };
        {
            let mut state = self.shared.lock();
            state.job = Some(Job { f: erased, count });
            state.posted += 1;
            state.left = count - 1;
            self.shared.wake.notify_all();
        }
        let mine = catch_unwind(AssertUnwindSafe(|| f(0)));
        let mut state = self.shared.lock();
        while state.left > 0 {
            state = self.shared.done.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.job = None;
        let theirs = state.panic.take();
        drop(state);
        if let Some(panic) = theirs.or(mine.err()) {
            resume_unwind(panic);
        }
    }

    /// A worker: each job posted that is for its index, run once, and
    /// between them parked.
    fn work(shared: &Shared, index: usize) {
        let mut seen = 0;
        let mut state = shared.lock();
        loop {
            if state.stop {
                return;
            }
            if state.posted == seen {
                state = shared.wake.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
                continue;
            }
            seen = state.posted;
            let Some(job) = state.job.filter(|job| index < job.count) else { continue };
            drop(state);
            // SAFETY: `Pool::run` keeps the closure alive until `left` is
            // 0, which it is not until this worker says so below.
            let f = unsafe { &*job.f };
            let outcome = catch_unwind(AssertUnwindSafe(|| f(index)));
            state = shared.lock();
            if let Err(panic) = outcome {
                state.panic.get_or_insert(panic);
            }
            state.left -= 1;
            if state.left == 0 {
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

    #[test]
    fn every_thread_runs_the_closure_with_its_own_index_and_the_caller_waits() {
        let pool = Pool::new(4);
        assert_eq!(pool.threads(), 4);
        for count in [1, 2, 4, 9] {
            let ran = Mutex::new(Vec::new());
            let sum = AtomicUsize::new(0);
            pool.run(count, |i| {
                std::thread::sleep(std::time::Duration::from_millis(5));
                sum.fetch_add(i, Ordering::Relaxed);
                ran.lock().unwrap().push(i);
            });
            let mut ran = ran.into_inner().unwrap();
            ran.sort_unstable();
            let n = count.min(4);
            assert_eq!(ran, (0..n).collect::<Vec<_>>(), "{count} asked for");
            assert_eq!(sum.into_inner(), n * (n - 1) / 2);
        }
    }

    #[test]
    fn a_borrowed_stack_is_written_and_read_back() {
        let pool = Pool::new(3);
        let mut out = [0u32; 3];
        let slots: Vec<Mutex<&mut u32>> = out.iter_mut().map(Mutex::new).collect();
        pool.run(3, |i| **slots[i].lock().unwrap() = i as u32 * 10 + 1);
        drop(slots);
        assert_eq!(out, [1, 11, 21]);
    }

    #[test]
    fn a_panic_on_any_thread_reaches_the_caller_after_the_rest_are_done() {
        let pool = Pool::new(3);
        for culprit in 0..3 {
            let finished = AtomicUsize::new(0);
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                pool.run(3, |i| {
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
            pool.run(3, |_| {
                count.fetch_add(1, Ordering::Relaxed);
            });
            assert_eq!(count.into_inner(), 3);
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
