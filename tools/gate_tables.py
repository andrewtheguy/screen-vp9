"""The tables of docs/rust-encoder-plan.md, "Where it stands on real captures", from
the log of `cargo test --release captures_against_libvpx -- --ignored --nocapture`,
followed, if the log holds it, by that of `settles_against_libvpx`:

    python3 -I tools/gate_tables.py <log>

Per capture and size the bytes ratio, the median time ratio and the luma PSNR
difference over every frame; then the frames apart, the keyframes, and the
settles from dial 65 to 90.
"""
import re, sys, collections
log = open(sys.argv[1]).read()
sizes1 = ["1280x800", "1440x900", "1600x1000", "1920x1080"]
sizes2 = ["2048x1536", "2560x1600", "2880x1800", "3200x2000", "3456x2168", "3840x2160"]
caps = ["terminal", "uncover", "flood", "browser", "drag", "video", "walk"]
# capture -> size -> kind -> (lbytes, lms, ldb, rbytes, rms, rdb, bytes_ratio, time_ratio, dbdiff, n)
data = collections.defaultdict(dict)
# capture -> size -> "from:to" -> (bytes_ratio, time_ratio, dbdiff)
settles = collections.defaultdict(dict)
cur = None
for line in log.splitlines():
    m = re.match(r"^([a-z]+)-(\d+x\d+)(?:@2)? (\d+x\d+) on \d+(, (\d+) frames)?", line)
    if m:
        cur = (m.group(1), m.group(2))
        if m.group(4): data[cur] = {}
        continue
    m = re.match(r'^  (key|whole|small|mid|large|all)\s+(\d+) \| libvpx\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| rust\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| bytes ([\d.]+) time median ([\d.]+)', line)
    if m and cur:
        k = m.group(1); v = [float(x) for x in m.groups()[1:]]
        data[cur][k] = dict(n=int(v[0]), lb=v[1], lms=v[2], ldb=v[3], rb=v[4], rms=v[5], rdb=v[6], bytes=v[7], time=v[8])
        continue
    m = re.match(r'^  q(\d+:\d+)\s+\| libvpx .* \| bytes ([\d.]+) time ([\d.]+) dB ([-+][\d.]+)', line)
    if m and cur:
        settles[cur][m.group(1)] = tuple(float(x) for x in m.groups()[1:])
def cell_all(c, s):
    d = data.get((c, s), {}).get("all")
    if not d: return "—"
    return f"{d['bytes']:.2f} / {d['time']:.2f} / {d['rdb']-d['ldb']:+.1f}".replace("+0.0", "0.0").replace("-0.0", "0.0")
def cell_kind(c, s, k, kb=False):
    d = data.get((c, s), {}).get(k)
    if not d: return "—"
    lb, rb = d['lb']/d['n'], d['rb']/d['n']
    if kb or lb >= 10000: b = f"{lb/1000:.0f} → {rb/1000:.0f} kB"
    else: b = f"{lb:.0f} → {rb:.0f} B"
    ms = f"{d['lms']:.2f} → {d['rms']:.2f} ms" if d['lms'] < 10 else f"{d['lms']:.1f} → {d['rms']:.1f} ms"
    return f"{ms}, {b}, {d['rdb']-d['ldb']:+.1f} dB".replace("+0.0 dB", "0.0 dB").replace("-0.0 dB", "0.0 dB")
def cell_key(c, s):
    d = data.get((c, s), {}).get("key")
    if not d: return "—"
    return f"{d['lms']:.0f} → {d['rms']:.0f} ms, {d['bytes']:.2f}×, {d['rdb']-d['ldb']:+.1f} dB"
def table(title, sizes, cells, rows):
    print(f"| {title} | " + " | ".join(s.replace("x", "×") + ("" if sizes is sizes1 else "@2") for s in sizes) + " |")
    print("|---" * (len(sizes) + 1) + "|")
    for name, c, *args in rows:
        print(f"| {name} | " + " | ".join(cells(c, s, *args) for s in sizes) + " |")
    print()
for title, sizes in [("Capture", sizes1), ("Capture", sizes2)]:
    table(title, sizes, cell_all, [(f"`{c}`", c) for c in caps])
frames = [("`terminal`, a line of text (≈ 260 a capture)", "terminal", "small"), ("`terminal`, a scroll (7 or 8)", "terminal", "large"), ("`drag`, a quarter of the picture (≈ 390)", "drag", "large"), ("`walk`, the settle (1)", "walk", "whole")]
for title, sizes in [("Frames, scale 1", sizes1), ("Frames, scale 2", sizes2)]:
    table(title, sizes, cell_kind, frames)
keys = [("`uncover`, a page under a terminal", "uncover"), ("`drag`, a terminal of text", "drag"), ("`flood`", "flood"), ("`video`", "video")]
for title, sizes in [("Keyframes of, scale 1", sizes1), ("Keyframes of, scale 2", sizes2)]:
    table(title, sizes, cell_key, keys)
def cell_settle(c, s, pair):
    d = settles.get((c, s), {}).get(pair)
    if not d: return "—"
    return f"{d[0]:.2f} / {d[1]:.2f} / {d[2]:+.1f}".replace("+0.0", "0.0").replace("-0.0", "0.0")
if settles:
    for title, sizes in [("Settle from 65 to 90", sizes1), ("Settle from 65 to 90", sizes2)]:
        table(title, sizes, cell_settle, [(f"`{c}`", c, "65:90") for c in ["uncover", "flood", "drag", "video", "walk"]])
