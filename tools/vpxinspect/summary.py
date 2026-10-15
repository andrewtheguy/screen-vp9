"""What tools/vpxinspect/build.sh's decoder and sweep say, as tables:

    python3 -I tools/vpxinspect/summary.py <name> <vpxdec stderr> <name> <vpxdec stderr>
    python3 -I tools/vpxinspect/summary.py --sweep <variant> ...

The first sets two streams of the same frames side by side, the second over
the first: the bytes each kind of read cost over the stream, from the
decoder's `total` line, and what share of the picture's area its blocks
were, intra and inter, skipped, by mode, reference, vector, sub-pixel and
filter, block size and transform size. A stream's stderr is what
`dec/vpxdec --threads=1 --noblit <ivf> 2>file` leaves, the IVF one
captures_against_libvpx writes with `IVF` set.

The second, from tmp/vpxinspect/sweep-<variant>.log, per capture and size:
libvpx's bytes over stock libvpx's and its PSNR difference, and this
encoder's bytes over that libvpx's and its PSNR difference, over every
frame.
"""
import re
import sys

CATS = "header partition skip segment ref tx intra_mode inter_mode filter mv coef_intra coef_inter coef_intra_uv coef_inter_uv".split()
BSIZES = "4x4 4x8 8x4 8x8 8x16 16x8 16x16 16x32 32x16 32x32 32x64 64x32 64x64".split()
TX = "4x4 8x8 16x16 32x32".split()


def parse(line):
    words = line.split()
    d, i = {}, 1 if words[0] == "total" else 2
    while i < len(words):
        w = words[i]
        if w == "|":
            i += 1
        elif w.endswith("_bsize") or w.endswith("_tx"):
            n = len(BSIZES) if w.endswith("_bsize") else len(TX)
            d[w] = [float(x) for x in words[i + 1 : i + 1 + n]]
            i += 1 + n
        else:
            d[w] = float(words[i + 1])
            i += 2
    return d


def total(path):
    for line in open(path):
        if line.startswith("total "):
            return parse(line)
    sys.exit(f"{path}: no total line")


def streams(args):
    names, s = args[0::2], [total(p) for p in args[1::2]]
    print(f"{'':22}" + "".join(f"{n:>14}" for n in names) + f"{'ratio':>8}")

    def row(label, vals, fmt="{:>14.0f}"):
        print(f"{label:22}" + "".join(fmt.format(v) for v in vals) + (f"{vals[1] / vals[0]:>8.2f}" if vals[0] else ""))

    row("bytes", [x["bytes"] for x in s])
    row("qindex", [x["q"] for x in s], "{:>14.1f}")
    for c in CATS:
        row(f"  {c} B", [x[c] for x in s])
    area = s[0]["area"]

    def pct(label, value):
        row(label, [100 * value(x) / area for x in s], "{:>13.1f}%")

    print("of the area:")
    for label, key in [("intra", "intra"), ("intra skipped", "skip_intra"), ("inter", "inter"), ("inter skipped", "skip_inter"), ("nearestmv", "nearest"), ("nearmv", "near"), ("zeromv", "zero"), ("newmv", "new"), ("golden", "golden"), ("altref", "altref"), ("compound", "compound"), ("moved", "moved"), ("sub-pixel", "subpel"), ("half-pixel", "half"), ("sub-pixel regular", "regular"), ("sub-pixel smooth", "smooth"), ("sub-pixel sharp", "sharp"), ("sub-pixel bilinear", "bilinear")]:
        pct(f"  {label}", lambda x, key=key: x[key])
    for kind in ["intra", "inter"]:
        for what, names in [("bsize", BSIZES), ("tx", TX)]:
            for i, b in enumerate(names):
                if any(x[f"{kind}_{what}"][i] for x in s):
                    pct(f"  {kind} {'tx ' if what == 'tx' else ''}{b}", lambda x, i=i, k=f"{kind}_{what}": x[k][i])


def sweep(variants):
    data = {}
    for v in variants:
        name = None
        for line in open(f"tmp/vpxinspect/sweep-{v}.log"):
            m = re.match(r"^(\S+) \d+x\d+ on", line)
            if m:
                name = m.group(1)
            m = re.match(r"^  all\s+\d+ \| libvpx\s+(\d+) B\s+\S+ ms\s+(\S+) dB \| rust\s+(\d+) B\s+\S+ ms\s+(\S+) dB", line)
            if m:
                data.setdefault(name, {})[v] = tuple(float(x) for x in m.groups())
    print(f"{'':24}" + "".join(f"{v:>26}" for v in variants))
    print(f"{'':24}" + "".join(f"{'libvpx':>13}{'rust':>13}" for _ in variants))
    for name in sorted(data):
        d, cells = data[name], []
        stock = d.get("stock", d[variants[0]])
        for v in variants:
            lb, lp, rb, rp = d[v]
            cells.append(f"{lb / stock[0]:6.3f} {lp - stock[1]:+5.2f}  {rb / lb:5.2f} {rp - lp:+5.2f}")
        print(f"{name:24}" + "".join(f"{c:>26}" for c in cells))


if __name__ == "__main__":
    if sys.argv[1] == "--sweep":
        sweep(sys.argv[2:])
    else:
        streams(sys.argv[1:])
