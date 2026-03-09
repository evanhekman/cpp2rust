#!/usr/bin/env python3
"""
Benchmark the synthesizer across all targets.
Runs each target N times and reports mean, min, max.
"""

import argparse
import json
import os
import subprocess
import sys
import time


def run_once(target, max_depth, timeout, dataset):
    start = time.perf_counter()
    result = subprocess.run(
        [
            sys.executable, "synthesize.py",
            "--target", target,
            "--max-depth", str(max_depth),
            "--timeout", str(timeout),
            "--dataset", dataset,
        ],
        capture_output=True,
        text=True,
    )
    elapsed = time.perf_counter() - start
    found = "FOUND" in result.stdout
    return elapsed, found


def main():
    parser = argparse.ArgumentParser(description="Synthesizer benchmark")
    parser.add_argument("--dataset", default="synthesizer/dataset")
    parser.add_argument("--max-depth", type=int, default=8)
    parser.add_argument("--timeout", type=int, default=30)
    parser.add_argument("--runs", type=int, default=3, help="Runs per target")
    parser.add_argument("--targets", nargs="*", help="Specific targets (default: all)")
    parser.add_argument("--output", default=None, help="Save results to JSON file")
    args = parser.parse_args()

    available = sorted(
        f[:-5] for f in os.listdir(args.dataset) if f.endswith(".json")
    )
    targets = args.targets if args.targets else available

    print(f"Benchmarking {len(targets)} targets, {args.runs} run(s) each")
    print(f"max-depth={args.max_depth}  timeout={args.timeout}s\n")
    print(f"{'target':<16} {'status':<8} {'mean':>8} {'min':>8} {'max':>8}")
    print("-" * 52)

    results = {}
    for target in targets:
        times = []
        found = None
        for _ in range(args.runs):
            elapsed, ok = run_once(target, args.max_depth, args.timeout, args.dataset)
            times.append(round(elapsed, 2))
            found = ok

        mean = round(sum(times) / len(times), 2)
        status = "FOUND" if found else "TIMEOUT"
        print(f"{target:<16} {status:<8} {mean:>7.2f}s {min(times):>7.2f}s {max(times):>7.2f}s")
        results[target] = {"status": status, "times": times, "mean": mean}

    print()
    found_targets = [t for t, r in results.items() if r["status"] == "FOUND"]
    print(f"Solved: {len(found_targets)}/{len(targets)}")
    if found_targets:
        total_mean = round(sum(results[t]["mean"] for t in found_targets) / len(found_targets), 2)
        print(f"Mean time (solved only): {total_mean}s")

    if args.output:
        with open(args.output, "w") as f:
            json.dump({"config": vars(args), "results": results}, f, indent=2)
        print(f"\nResults saved to {args.output}")


if __name__ == "__main__":
    main()
