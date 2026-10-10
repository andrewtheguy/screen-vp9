"""The tables of docs/benchmarks.md, from the logs of the three measurements
in src/encoder/mod.rs, each run in release with `--ignored --nocapture`:

    python3 -I tools/tables.py <captures log> [<keyframes log> [<settles log>]]

From `captures_against_libvpx`: per capture and size the bytes ratio, the
median of the per-frame time ratios and the luma PSNR difference over every
frame, a table each; then the frames apart. From `keyframes_against_libvpx`
and `settles_against_libvpx`: per dial or pair of dials, a table each of the
time ratio, the bytes ratio and the PSNR difference. A ratio is this encoder
over libvpx; green is this encoder's side of the line.
"""
import collections
import re
import sys

SIZES = ["1280x800", "1440x900", "1600x1000", "1920x1080", "2048x1536", "2560x1600", "2880x1800", "3200x2000", "3456x2168", "3840x2160"]
SCALE2 = set(SIZES[4:])
CAPTURES = ["terminal", "uncover", "flood", "browser", "drag", "video", "walk"]
HEAD = re.compile(r"^([a-z]+)-(\d+x\d+)(?:@2)? (\d+x\d+) on \d+(, \d+ frames)?")
KIND = re.compile(r"^  (key|whole|small|mid|large|all)\s+(\d+) \| libvpx\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| rust\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| bytes ([\d.]+) time median ([\d.]+) total ([\d.]+)")
PAIR = re.compile(r"^  q([\d:]+)\s+\| libvpx\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| rust\s+(\d+) B\s+([\d.]+) ms\s+([\d.]+) dB \| bytes ([\d.]+) time ([\d.]+) dB ([-+][\d.]+) all ([-+][\d.]+)")


def read(path):
    """capture -> size -> label -> the numbers of its line; and for a
    keyframe or settle log the size of the picture each was made of."""
    data, made_of, cur = collections.defaultdict(dict), {}, None
    for line in open(path):
        m = HEAD.match(line)
        if m:
            cur = (m.group(1), m.group(2))
            made_of[cur] = m.group(3)
            data.setdefault(cur, {})
            continue
        m = KIND.match(line)
        if m and cur:
            v = [float(x) for x in m.groups()[1:]]
            data[cur][m.group(1)] = dict(n=int(v[0]), lb=v[1], lms=v[2], ldb=v[3], rb=v[4], rms=v[5], rdb=v[6], bytes=v[7], time=v[8], total=v[9], db=v[6] - v[3])
            continue
        m = PAIR.match(line)
        if m and cur:
            v = [float(x) for x in m.groups()[1:]]
            data[cur][m.group(1)] = dict(lb=v[0], lms=v[1], ldb=v[2], rb=v[3], rms=v[4], rdb=v[5], bytes=v[6], time=v[7], db=v[8], all=v[9])
    return data, made_of


def colour(text, good):
    if good is None:
        return text
    return "$\\color{%s}{%s}$" % ("green" if good else "red", text)


def ratio(v):
    text = f"{v:.2f}"
    return colour(text, None if text == "1.00" else v < 1)


def decibels(v):
    text = f"{v:+.1f}"
    if text in ("+0.0", "-0.0"):
        return "0.0"
    return colour(text, v > 0)


def size_name(s):
    return s.replace("x", "×") + ("@2" if s in SCALE2 else "")


def grid(title, note, data, label, field, show, rows=CAPTURES):
    sizes = [s for s in SIZES if any(label in data.get((c, s), {}) for c in rows)]
    if not sizes:
        return
    print(f"### {title}\n\n{note}\n")
    print("| Capture | " + " | ".join(size_name(s) for s in sizes) + " |")
    print("|---|" + "---:|" * len(sizes))
    for c in rows:
        cells = [show(data[(c, s)][label][field]) if label in data.get((c, s), {}) else "—" for s in sizes]
        if any(cell != "—" for cell in cells):
            print(f"| `{c}` | " + " | ".join(cells) + " |")
    print()


def amount(b):
    return f"{b / 1000:.0f} kB" if b >= 10000 else f"{b:.0f} B"


def ms(v):
    return f"{v:.2f}" if v < 10 else f"{v:.1f}"


def apart(title, data, capture, kind):
    rows = [(s, data[(capture, s)][kind]) for s in SIZES if kind in data.get((capture, s), {})]
    if not rows:
        return
    print(f"### {title}\n")
    print("| Size | Frames | libvpx, ms | This encoder, ms | Time ratio | libvpx, bytes | This encoder, bytes | Bytes ratio | PSNR difference, dB |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for s, d in rows:
        lb, rb = d["lb"] / d["n"], d["rb"] / d["n"]
        mine = colour(ms(d["rms"]), None if ms(d["rms"]) == ms(d["lms"]) else d["rms"] < d["lms"])
        print(f"| {size_name(s)} | {d['n']} | {ms(d['lms'])} | {mine} | {ratio(d['time'])} | {amount(lb)} | {amount(rb)} | {ratio(d['bytes'])} | {decibels(d['db'])} |")
    print()


def main():
    data, _ = read(sys.argv[1])
    print("## Against libvpx, every capture at every size\n")
    grid("Bytes over the whole capture, this encoder over libvpx", "Under 1.00 is fewer bytes than libvpx.", data, "all", "bytes", ratio)
    grid("Encode time, the median of the per-frame ratios", "Under 1.00 is faster than libvpx, frame for frame.", data, "all", "time", ratio)
    grid("Encode time, the whole capture's", "Every frame's time summed, this encoder's over libvpx's: where the long frames weigh.", data, "all", "total", ratio)
    grid("Luma PSNR, this encoder less libvpx, in dB", "Above zero is a better picture than libvpx at the same dial.", data, "all", "db", decibels)
    print("## Against libvpx, the frames apart\n")
    print("The time columns are each encoder's median time for a frame of that kind, the\ntime ratio the median of the per-frame ratios, and the bytes the mean per frame.\n")
    apart("`terminal`, a line of text", data, "terminal", "small")
    apart("`terminal`, a scroll", data, "terminal", "large")
    apart("`drag`, a quarter of the picture", data, "drag", "large")
    apart("`browser`, a page scrolled", data, "browser", "large")
    apart("`flood`, a window of new text", data, "flood", "large")
    apart("`uncover`, a window moved off a page", data, "uncover", "large")
    apart("`video`, a frame of the pattern", data, "video", "large")
    apart("`walk`, a frame as the dial walks", data, "walk", "large")
    apart("`walk`, the settle a session asked for", data, "walk", "whole")
    for c in ["uncover", "drag", "flood", "video"]:
        apart(f"Keyframes inside `{c}`", data, c, "key")
    for path, what, which in zip(sys.argv[2:], ["A keyframe at dial", "A settle from dial"], ["keyframes", "settles"]):
        pairs, made_of = read(path)
        # A first picture that is not the capture's size is the desktop
        # before its resize, and says nothing of the size.
        rows = [c for c in CAPTURES if any(made_of.get((c, s)) == s for s in SIZES)]
        pairs = {k: v for k, v in pairs.items() if made_of.get(k) == k[1]}
        labels = []
        for v in pairs.values():
            labels += [label for label in v if label not in labels]
        print(f"## Against libvpx, {which} of each capture's first picture\n")
        for label in labels:
            name = f"{what} {label.replace(':', ' to ')}"
            grid(f"{name}: time", "This encoder's least time over libvpx's.", pairs, label, "time", ratio, rows)
            grid(f"{name}: bytes", "This encoder's bytes over libvpx's.", pairs, label, "bytes", ratio, rows)
            grid(f"{name}: luma PSNR", "This encoder's less libvpx's, in dB.", pairs, label, "db", decibels, rows)


main()
