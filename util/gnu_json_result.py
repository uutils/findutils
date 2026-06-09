#!/usr/bin/env python3

"""
Build a per-test JSON summary of a GNU findutils `make check` run.

GNU findutils runs tests through three harnesses that each log results
differently:

  * dejagnu  -> find/testsuite/find.log and xargs/testsuite/xargs.log
                ("PASS: <name>" / "FAIL: <name>, <reason>" lines)
  * automake -> tests/**/*.log, one log per test script, each ending with a
                "<STATUS> tests/<path>.sh (exit status: N)" line

The names are naturally disjoint (dejagnu find names end in ".new-O[0-3]",
automake names start with "tests/"), so they can share one flat namespace.

Output format (consumed by compare_test_results.py):

    {
      "summary": {"total": N, "passed": P, "failed": F, "skipped": S},
      "tests": [{"name": "...", "status": "PASS|FAIL|SKIP"}, ...]
    }
"""

import json
import re
import sys
from pathlib import Path

# dejagnu status -> normalized status. Anything unexpected counts as FAIL so a
# regression is never silently dropped.
DEJAGNU = {
    "PASS": "PASS",
    "XFAIL": "PASS",  # expected failure: not a regression
    "FAIL": "FAIL",
    "XPASS": "FAIL",  # unexpected pass: worth surfacing
    "ERROR": "FAIL",
    "UNRESOLVED": "FAIL",
    "UNSUPPORTED": "SKIP",
    "UNTESTED": "SKIP",
}

DEJAGNU_RE = re.compile(r"^(PASS|XFAIL|FAIL|XPASS|ERROR|UNRESOLVED|UNSUPPORTED|UNTESTED): (.+)$")
# automake per-test trailer, e.g. "FAIL tests/find/used.sh (exit status: 1)"
AUTOMAKE_RE = re.compile(r"^(PASS|FAIL|SKIP|XPASS|XFAIL|ERROR) (tests/\S+?)(?:\.sh)? \(exit status: \d+\)$")
AUTOMAKE = {
    "PASS": "PASS",
    "XFAIL": "PASS",
    "FAIL": "FAIL",
    "XPASS": "FAIL",
    "ERROR": "FAIL",
    "SKIP": "SKIP",
}


def _read(path):
    return path.read_text(encoding="utf-8", errors="replace").splitlines()


def _record(tests, name, status):
    """Merge a status into `tests`, keeping failure sticky.

    DejaGnu emits one line per assertion, so a single test name can appear many
    times with mixed results. A test counts as FAIL if any assertion failed,
    else PASS if any passed, else SKIP.
    """
    prev = tests.get(name)
    if prev == "FAIL" or status == "FAIL":
        tests[name] = "FAIL"
    elif prev == "PASS" or status == "PASS":
        tests[name] = "PASS"
    else:
        tests[name] = "SKIP"


def collect(root):
    """Return {name: status} for every test found under `root`."""
    root = Path(root)
    tests = {}

    # dejagnu logs
    for rel in ("find/testsuite/find.log", "xargs/testsuite/xargs.log"):
        log = root / rel
        if not log.is_file():
            continue
        for line in _read(log):
            m = DEJAGNU_RE.match(line)
            if not m:
                continue
            status, rest = m.group(1), m.group(2)
            # FAIL lines carry a trailing ", <reason>"; the name is the head.
            name = rest.split(",", 1)[0].strip()
            _record(tests, name, DEJAGNU[status])

    # automake per-test logs (skip the aggregate test-suite.log)
    for log in (root / "tests").rglob("*.log"):
        if log.name == "test-suite.log":
            continue
        for line in _read(log):
            m = AUTOMAKE_RE.match(line)
            if m:
                _record(tests, m.group(2), AUTOMAKE[m.group(1)])
                break

    return tests


def build(root):
    tests = collect(root)
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
        print(f"usage: {sys.argv[0]} <gnu-source-dir> <output.json>", file=sys.stderr)
        return 2
    result = build(sys.argv[1])
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, sort_keys=True)
        f.write("\n")
    s = result["summary"]
    print(
        f"GNU findutils tests summary = TOTAL: {s['total']} / "
        f"PASS: {s['passed']} / FAIL: {s['failed']} / SKIP: {s['skipped']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
