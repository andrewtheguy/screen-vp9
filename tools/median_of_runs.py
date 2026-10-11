"""One log of a measurement from several runs of it, for tools/tables.py,
where a machine's times move too much from run to run to be read off one:

    python3 -I tools/median_of_runs.py <out log> <run log> <run log> <run log> ...

Each line of a capture, a keyframe or a settle is taken from the run whose
time ratio is the median of the runs' for that line; the runs must be of the
same measurement over the same captures. The test harness's own words in
front of the first capture are dropped.
"""
import re
import sys

TIME = re.compile(r"\| bytes [\d.]+ time (?:median )?([\d.]+)")
HARNESS = re.compile(r"^test \S+ \.\.\. ")


def main():
    out, paths = sys.argv[1], sys.argv[2:]
    runs = [[HARNESS.sub("", line) for line in open(path).read().splitlines()] for path in paths]
    if len({len(run) for run in runs}) != 1:
        sys.exit("the runs are not of the same measurement")
    lines = []
    for same in zip(*runs):
        if TIME.search(same[0]):
            same = sorted(same, key=lambda line: float(TIME.search(line).group(1)))
            lines.append(same[len(same) // 2])
        else:
            lines.append(same[0])
    open(out, "w").write("\n".join(lines) + "\n")


main()
