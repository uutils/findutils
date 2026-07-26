#!/usr/bin/env python3

"""
Compare the current GNU test results to the reference results gathered from the
main branch, to highlight whether a PR makes the results better or worse.

Writes a human-readable comparison to --output (empty when nothing changed, so
the comment workflow can decide to stay silent). Exits 1 when there are new,
non-intermittent failures; intermittent (flaky) tests listed in --ignore-file
are reported but never fail the job.

Adapted from the uutils sed/grep workflow.
"""

import json
import sys
import argparse
from pathlib import Path


def load_ignore_list(ignore_file):
    """Load the set of intermittent test names to ignore from a file."""
    ignore_set = set()
    if ignore_file and Path(ignore_file).exists():
        with open(ignore_file, "r") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#"):
                    ignore_set.add(line)
    return ignore_set


def extract_test_results(json_data):
    """Return (summary, failed_test_names) from parsed JSON data."""
    if not json_data or "summary" not in json_data:
        return {"total": 0, "passed": 0, "failed": 0, "skipped": 0}, []

    summary = json_data["summary"]
    failed_tests = [
        test.get("name", "unknown")
        for test in json_data.get("tests", [])
        if test.get("status") == "FAIL"
    ]
    return summary, failed_tests


def compare_results(current_file, reference_file, ignore_file=None, output_file=None):
    """Compare current results with reference results."""
    ignore_set = load_ignore_list(ignore_file)

    try:
        with open(current_file, "r") as f:
            current_summary, current_failed = extract_test_results(json.load(f))
    except Exception as e:
        print(f"Error loading current results: {e}")
        return 1

    try:
        with open(reference_file, "r") as f:
            reference_summary, reference_failed = extract_test_results(json.load(f))
    except Exception as e:
        print(f"Error loading reference results: {e}")
        return 1

    pass_diff = int(current_summary.get("passed", 0)) - int(
        reference_summary.get("passed", 0)
    )
    fail_diff = int(current_summary.get("failed", 0)) - int(
        reference_summary.get("failed", 0)
    )
    total_diff = int(current_summary.get("total", 0)) - int(
        reference_summary.get("total", 0)
    )

    current_failed_set = set(current_failed)
    reference_failed_set = set(reference_failed)

    new_failures = current_failed_set - reference_failed_set
    improvements = reference_failed_set - current_failed_set

    non_intermittent_new_failures = new_failures - ignore_set

    no_changes = (
        pass_diff == 0
        and fail_diff == 0
        and total_diff == 0
        and not new_failures
        and not improvements
    )

    # Empty output tells the comment workflow there is nothing to post.
    if no_changes:
        if output_file:
            with open(output_file, "w") as f:
                f.write("")
        return 0

    output_lines = []
    output_lines.append("Test results comparison:")
    output_lines.append(
        f"  Current:   TOTAL: {current_summary.get('total', 0)} / PASSED: {current_summary.get('passed', 0)} / FAILED: {current_summary.get('failed', 0)} / SKIPPED: {current_summary.get('skipped', 0)}"
    )
    output_lines.append(
        f"  Reference: TOTAL: {reference_summary.get('total', 0)} / PASSED: {reference_summary.get('passed', 0)} / FAILED: {reference_summary.get('failed', 0)} / SKIPPED: {reference_summary.get('skipped', 0)}"
    )
    output_lines.append("")

    if pass_diff != 0 or fail_diff != 0 or total_diff != 0:
        output_lines.append("Changes from main branch:")
        output_lines.append(f"  TOTAL: {total_diff:+d}")
        output_lines.append(f"  PASSED: {pass_diff:+d}")
        output_lines.append(f"  FAILED: {fail_diff:+d}")
        output_lines.append("")

    if new_failures:
        # Only non-intermittent failures fail the job, but list them all.
        real = sorted(new_failures - ignore_set)
        flaky = sorted(new_failures & ignore_set)
        output_lines.append(f"New test failures ({len(new_failures)}):")
        for test in real:
            output_lines.append(f"  - {test}")
        for test in flaky:
            output_lines.append(f"  - {test} (intermittent)")
        output_lines.append("")

    if improvements:
        output_lines.append(f"Test improvements ({len(improvements)}):")
        for test in sorted(improvements):
            output_lines.append(f"  + {test}")
        output_lines.append("")

    output_text = "\n".join(output_lines)
    if output_file:
        with open(output_file, "w") as f:
            f.write(output_text)
    else:
        print(output_text)

    if non_intermittent_new_failures:
        print(
            f"ERROR: Found {len(non_intermittent_new_failures)} new non-intermittent test failures"
        )
        return 1

    return 0


def main():
    parser = argparse.ArgumentParser(description="Compare GNU test results")
    parser.add_argument("current", help="Current test results JSON file")
    parser.add_argument("reference", help="Reference test results JSON file")
    parser.add_argument(
        "--ignore-file", help="File containing intermittent test names to ignore"
    )
    parser.add_argument("--output", help="Output file for comparison results")

    args = parser.parse_args()
    return compare_results(args.current, args.reference, args.ignore_file, args.output)


if __name__ == "__main__":
    sys.exit(main())
