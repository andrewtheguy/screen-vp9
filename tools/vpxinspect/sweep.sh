#!/usr/bin/env bash
# captures_against_libvpx against the libvpx tools/vpxinspect/build.sh built,
# its encoder cut down as each variant names, a log of each:
#
#   tools/vpxinspect/sweep.sh [VARIANT ...]
#
# stock (libvpx as screen-vp9 sets it up), whole (whole-pixel vectors
# alone), half (to a half pixel), quarter-regular and half-regular (the
# regular filter alone), half-smooth, half-bilinear, nogolden (the last frame
# the one reference). Each runs the captures `ONLYS` names (`video walk`)
# from `CAPTURES` (tmp/samples) into tmp/vpxinspect/sweep-<variant>.log,
# appended to; tools/vpxinspect/summary.py --sweep reads them. Bytes and
# PSNR, not time: the variants are run one after another.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
export CARGO_TARGET_DIR=tmp/vpxinspect/target LIBVPX_PREBUILT_DIR="$root/tmp/vpxinspect/prefix" CAPTURES="${CAPTURES:-tmp/samples}"
cargo test --release --features native --lib captures_against_libvpx --no-run 2>/dev/null
for variant in "${@:-stock}"; do
	case "$variant" in
		stock) env=() ;;
		whole) env=(VPXI_SUBPEL=3) ;;
		half) env=(VPXI_SUBPEL=2) ;;
		quarter-regular) env=(VPXI_SUBPEL=1 VPXI_FILTER=0) ;;
		half-regular) env=(VPXI_SUBPEL=2 VPXI_FILTER=0) ;;
		half-smooth) env=(VPXI_SUBPEL=2 VPXI_FILTER=1) ;;
		half-bilinear) env=(VPXI_SUBPEL=2 VPXI_FILTER=3) ;;
		nogolden) env=(VPXI_NOGOLDEN=1) ;;
		*) echo "$0: $variant: not a variant" >&2; exit 2 ;;
	esac
	for only in ${ONLYS:-video walk}; do
		echo "$(date +%T) $variant $only" >&2
		env "${env[@]}" ONLY="$only" cargo test --release --features native --lib captures_against_libvpx -- --ignored --nocapture 2>/dev/null \
			| grep -vE '^(running|test |$)' >>"tmp/vpxinspect/sweep-$variant.log"
	done
done
