"""Write src/encoder/dct.rs from a libvpx checkout.

    python3 -I tools/vp9_dct.py LIBVPX_DIR > src/encoder/dct.rs

The one-dimensional inverse DCTs a decoder runs, and the 32-point forward DCT,
are long straight-line C. They are translated statement by statement rather
than retyped: casts dropped, the rounding helpers renamed, constant indices
folded, and every store into what libvpx holds as 16 bits wrapped to 16 bits,
as libvpx built without high bit depth does. The translation is generic over
`Lanes`, so one value or a vector of them goes through the same statements.
"""

import re
import subprocess
import sys


def function(source, name):
    start = source.index(name + "(")
    i = source.index("{", start)
    depth, body_start = 0, i + 1
    while True:
        depth += {"{": 1, "}": -1}.get(source[i], 0)
        if depth == 0:
            return source[body_start:i]
        i += 1


def strip_outer(expr):
    """`expr` without parentheses that enclose all of it."""
    while expr.startswith("(") and expr.endswith(")"):
        depth = 0
        for i, ch in enumerate(expr):
            depth += {"(": 1, ")": -1}.get(ch, 0)
            if depth == 0 and i < len(expr) - 1:
                return expr
        expr = expr[1:-1].strip()
    return expr


def translate(body, wrap):
    out = []
    statement = ""
    skip_depth = 0
    for raw in body.splitlines():
        line = raw.strip()
        if skip_depth:
            skip_depth += line.count("{") - line.count("}")
            continue
        if line.startswith("if (round)"):
            skip_depth = 1
            continue
        if not line or re.match(r"(int16_t|tran_high_t|tran_low_t|int) ", line):
            continue
        if line.startswith("//"):
            out.append("    " + line)
            continue
        statement += (" " if statement else "") + line
        if not statement.endswith(";"):
            continue
        lhs, rhs = (s.strip() for s in statement[:-1].split("=", 1))
        statement = ""
        rhs = re.sub(r"\((int16_t|tran_low_t|tran_high_t)\)", "", rhs)
        rhs = rhs.replace("WRAPLOW(", "(")
        rhs = re.sub(r"\b(dct_const_round_shift|fdct_round_shift|dct_32_round)\(", "round14(", rhs)
        rhs = re.sub(r"\bcospi_(\d+)_64\b", r"COSPI[\1]", rhs)
        fold = lambda m: f"[{eval(m.group(1).replace('/', '//'))}]"
        rhs = re.sub(r"\[([0-9 +\-*/()]+)\]", fold, rhs)
        lhs = re.sub(r"\[([0-9 +\-*/()]+)\]", fold, lhs)
        rhs = strip_outer(rhs)
        if wrap and re.match(r"(step\d?|output)\[", lhs):
            rhs = f"w({rhs})"
        out.append(f"    {lhs} = {rhs};")
    return out


def emit(source, c_name, rust_name, n, wrap, doc):
    body = translate(function(source, c_name), wrap)
    text = "\n".join(body)
    print()
    for line in doc.strip().splitlines():
        print(f"/// {line}".rstrip())
    print("#[inline(always)]")
    print(f"pub(super) fn {rust_name}<V: Lanes>(input: &[V; {n}], output: &mut [V; {n}]) {{")
    for array in ("step", "step1", "step2", "step3"):
        if re.search(rf"\b{array}\[", text):
            print(f"    let mut {array} = [V::splat(0); {n}];")
    for temp in ("temp1", "temp2"):
        if re.search(rf"\b{temp}\b", text):
            print(f"    let mut {temp}: V;")
    print(text)
    print("}")


def main():
    root = sys.argv[1]
    commit = subprocess.run(["git", "-C", root, "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    inv = open(f"{root}/vpx_dsp/inv_txfm.c").read()
    fwd = open(f"{root}/vpx_dsp/fwd_txfm.c").read()
    common = open(f"{root}/vpx_dsp/txfm_common.h").read()
    cospi = dict((int(i), int(v)) for i, v in re.findall(r"cospi_(\d+)_64 = (\d+);", common))

    print("//! VP9's one-dimensional DCTs, translated by `tools/vp9_dct.py` from libvpx at")
    print(f"//! commit `{commit}`; not edited by hand. Copyright the WebM")
    print("//! Project authors, under the BSD licence in `LICENSE-libvpx`.")
    print("//!")
    print("//! The inverse ones are what a decoder runs, so they are exact to the bit:")
    print("//! every value libvpx keeps in 16 bits is wrapped to 16 bits here. Each is")
    print("//! written over `Lanes`, so it runs on one value or on a vector of them.")
    print()
    print("use super::lanes::Lanes;")
    print()
    print("/// cos(kπ/64) in 14 bits, `COSPI[k]`.")
    print("const COSPI: [i32; 32] = [0, " + ", ".join(str(cospi[k]) for k in range(1, 32)) + "];")
    print()
    print("/// libvpx's `dct_const_round_shift`: a product of a `COSPI` back to scale.")
    print("#[inline(always)]")
    print("fn round14<V: Lanes>(x: V) -> V {")
    print("    x.round14()")
    print("}")
    print()
    print("/// A store into a 16-bit value.")
    print("#[inline(always)]")
    print("fn w<V: Lanes>(x: V) -> V {")
    print("    x.wrap16()")
    print("}")
    for n in (4, 8, 16, 32):
        emit(inv, f"void idct{n}_c", f"idct{n}", n, True, f"libvpx's `idct{n}_c`.")
    emit(fwd, "void vpx_fdct32", "fdct32", 32, False, "libvpx's `vpx_fdct32`, without the rounding its rate-distortion\nvariant asks for.")


main()
