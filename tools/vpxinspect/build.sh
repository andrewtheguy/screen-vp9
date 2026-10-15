#!/usr/bin/env bash
# libvpx 1.16.0, the commit libvpx-prebuilt pins, with libvpx-1.16.0.patch
# beside this script: an inspection build, for reading where a stream's bytes
# go and what libvpx's encoder makes without the sub-pixel search, its
# filters or its golden frame.
#
#   tools/vpxinspect/build.sh [DIR]
#
# Into DIR (tmp/vpxinspect): `src/`, the patched source; `dec/vpxdec`, a
# decoder that prints, on stderr as it exits, what each kind of read cost
# over the stream and what its blocks were (with `VPXI_FRAMES` set, each
# frame too; tools/vpxinspect/summary.py reads it); and `prefix/`, a libvpx
# for `LIBVPX_PREBUILT_DIR`, whose encoder reads `VPXI_SUBPEL` (how far its
# sub-pixel search goes: 0 an eighth, 1 a quarter, 2 a half, 3 whole pixels
# alone), `VPXI_FILTER` (one filter for every block: 0 regular, 1 smooth,
# 2 sharp, 3 bilinear) and `VPXI_NOGOLDEN` (the last frame the one
# reference), and is libvpx unchanged with none of them set.
# tools/vpxinspect/sweep.sh runs the harness against it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${1:-$root/tmp/vpxinspect}"
commit=1024874c5919305883187e2953de8fcb4c3d7fa6

mkdir -p "$out"
out="$(cd "$out" && pwd)"
if [[ ! -d "$out/src/.git" ]]; then
	git -c advice.detachedHead=false clone --quiet --branch v1.16.0 https://github.com/webmproject/libvpx "$out/src"
fi
[[ "$(git -C "$out/src" rev-parse HEAD)" == "$commit" ]] || { echo "$0: $out/src is not libvpx $commit" >&2; exit 1; }
if ! git -C "$out/src" apply --reverse --check "$here/libvpx-1.16.0.patch" 2>/dev/null; then
	git -C "$out/src" apply "$here/libvpx-1.16.0.patch"
fi

rm -rf "$out/dec" "$out/enc" "$out/prefix"
mkdir -p "$out/dec" "$out/enc"
(cd "$out/dec" && "$out/src/configure" --disable-vp8 --disable-vp9-encoder --disable-unit-tests --disable-docs --as=nasm >/dev/null && make -j"$(nproc)" >/dev/null)
# libvpx-prebuilt's build.sh's configure line for linux-x86_64.
(cd "$out/enc" && "$out/src/configure" --prefix="$out/prefix" --enable-static --disable-shared --enable-pic \
	--disable-examples --disable-tools --disable-docs --disable-unit-tests --disable-webm-io --disable-libyuv \
	--enable-realtime-only --disable-vp8 --enable-vp9-encoder --enable-vp9-decoder --target=x86_64-linux-gcc --as=nasm >/dev/null \
	&& make -j"$(nproc)" >/dev/null && make install >/dev/null)
echo "$out/dec/vpxdec"
echo "LIBVPX_PREBUILT_DIR=$out/prefix"
