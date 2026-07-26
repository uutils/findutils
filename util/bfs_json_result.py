#!/usr/bin/env python3

"""
Build a per-test JSON summary of a bfs testsuite run.

bfs writes one line per test to tests.log:

    [PASS] posix/H
    [FAIL] common/newermt
    [SKIP] gnu/...

Output format matches util/gnu_json_result.py and is consumed by
compare_test_results.py:

    {
      "summary": {"total": N, "passed": P, "failed": F, "skipped": S},
      "tests": [{"name": "...", "status": "PASS|FAIL|SKIP"}, ...]
    }
"""

import json
import re
import sys
from pathlib import Path

RESULT_RE = re.compile(r"^\[(PASS|FAIL|SKIP)\] (\S+)\s*$")


def collect(log_file):
    tests = {}
    log = Path(log_file)
    if log.is_file():
        for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
            m = RESULT_RE.match(line)
            if m:
                tests[m.group(2)] = m.group(1)
    return tests


def build(log_file):
    tests = collect(log_file)
    passed = sum(1 for s in tests.values() if s == "PASS")
    failed = sum(1 for s in tests.values() if s == "FAIL")
    skipped = sum(1 for s in tests.values() if s == "SKIP")
    return {
        "summary": {
            "total": len(tests),
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
        },
        "tests": [
            {"name": name, "status": status}
            for name, status in sorted(tests.items())
        ],
    }


def main():
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <bfs-tests.log> <output.json>", file=sys.stderr)
        return 2
    result = build(sys.argv[1])
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, sort_keys=True)
        f.write("\n")
    s = result["summary"]
    print(
        f"bfs tests summary = TOTAL: {s['total']} / "
        f"PASS: {s['passed']} / FAIL: {s['failed']} / SKIP: {s['skipped']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
