#!/usr/bin/env python3
"""
End-to-end pipeline: C++ → Preprocessor → Synthesizer → Stitch → verus_solver

Usage:
    python scripts/pipeline.py [--bench BENCH] [--targets t1 t2 ...]

Outputs a timing table showing pass/fail for each stage per target.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
REPO = Path(__file__).resolve().parent.parent          # cpp2rust/
VERUS_SOLVER_REPO = REPO.parent / "verus-proof-synthesis" / ".claude" / "worktrees" / "ecstatic-sammet"
VERUS_SOLVER_CONFIG = VERUS_SOLVER_REPO / "verus_solver" / "config.local.yaml"

CPP2JSON    = REPO / "target" / "release" / "cpp2json_cpp"
MAPJSON     = REPO / "target" / "release" / "map_cpp_json_to_rust_json"
SYNTH       = REPO / "target" / "release" / "synth"
VALIDATOR   = REPO / "target" / "release" / "transform_verus"
SYMBOLS     = REPO / "synthesizer" / "symbols.txt"

SKIP = {"graphs", "doubly_linekedlsit", "shared_mutable_aliasing"}

# ANSI colours
GRN = "\033[32m"
RED = "\033[31m"
YLW = "\033[33m"
RST = "\033[0m"
BLD = "\033[1m"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _run(cmd: list, cwd=None, timeout=300) -> tuple[bool, str, float]:
    """Run a command, return (ok, stdout+stderr, elapsed_sec)."""
    t0 = time.perf_counter()
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True, cwd=cwd, timeout=timeout,
        )
        elapsed = time.perf_counter() - t0
        out = r.stdout + r.stderr
        return r.returncode == 0, out, elapsed
    except subprocess.TimeoutExpired:
        elapsed = time.perf_counter() - t0
        return False, "TIMEOUT", elapsed
    except Exception as e:
        elapsed = time.perf_counter() - t0
        return False, str(e), elapsed


def _cell(ok: bool | None, elapsed: float | None) -> str:
    if ok is None:
        return f"{'—':^18}"
    sym = f"{GRN}✓{RST}" if ok else f"{RED}✗{RST}"
    t = f"{elapsed:.1f}s" if elapsed is not None else ""
    return f"  {sym}  {t:<8}    "


# ---------------------------------------------------------------------------
# Pipeline stages
# ---------------------------------------------------------------------------

def stage_preprocess(cpp_file: Path, out_json: Path) -> tuple[bool, str, float]:
    """C++ → processed JSON (two-step: cpp2json → mapjson)."""
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tf:
        tmp = Path(tf.name)
    try:
        ok, out, elapsed = _run([str(CPP2JSON), str(cpp_file), "--out", str(tmp)])
        if not ok:
            return False, out, elapsed
        ok2, out2, elapsed2 = _run([str(MAPJSON), str(tmp), "--out", str(out_json)])
        return ok2, out + out2, elapsed + elapsed2
    finally:
        tmp.unlink(missing_ok=True)


def stage_synthesize(json_file: Path, prepost_file: Path, stitched_file: Path) -> tuple[bool, str, float]:
    """
    processed JSON → stitched Rust (synth body + validator splice).
    Returns ok=True only if synthesis found a solution AND stitching succeeded.
    """
    t0 = time.perf_counter()

    # Run synth, capture stdout.
    r = subprocess.run(
        [str(SYNTH), "--file", str(json_file), "--symbols", str(SYMBOLS)],
        capture_output=True, text=True, timeout=300,
    )
    synth_elapsed = time.perf_counter() - t0
    synth_out = r.stdout + r.stderr

    if "FOUND" not in synth_out:
        return False, synth_out, synth_elapsed

    # Extract the synthesized code.
    # Format: "  FOUND in X.Xs  (N candidates, Z expansions):\n  <code>"
    # Everything after the FOUND line is the body.
    lines = synth_out.splitlines()
    found_idx = next((i for i, l in enumerate(lines) if "FOUND" in l and "candidates" in l), None)
    if found_idx is None:
        return False, synth_out, synth_elapsed

    # Code starts on the line(s) after FOUND.
    body_lines = lines[found_idx + 1:]
    # Strip the leading 2-space indent that synth adds.
    body = "\n".join(l[2:] if l.startswith("  ") else l for l in body_lines).strip()

    if not body:
        return False, synth_out + "\n(empty body extracted)", synth_elapsed

    # Write the impl to a temp file.
    with tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False) as tf:
        tf.write(f"fn __impl() {{\n{body}\n}}\n")
        impl_path = Path(tf.name)

    try:
        stitched_file.parent.mkdir(parents=True, exist_ok=True)
        ok, val_out, val_elapsed = _run([
            str(VALIDATOR),
            str(prepost_file),
            str(impl_path),
            str(stitched_file),
        ])
        total_elapsed = synth_elapsed + val_elapsed
        return ok, synth_out + "\n" + val_out, total_elapsed
    finally:
        impl_path.unlink(missing_ok=True)


def stage_verify(stitched_file: Path, validated_file: Path) -> tuple[bool, str, float]:
    """Stitched Rust → Verified Rust via verus_solver."""
    validated_file.parent.mkdir(parents=True, exist_ok=True)

    # Load the .env from the verus_solver repo so ANTHROPIC_API_KEY is available.
    env = os.environ.copy()
    env_file = VERUS_SOLVER_REPO / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, _, v = line.partition("=")
                env[k.strip()] = v.strip().strip('"').strip("'")

    cmd = [
        sys.executable, "-m", "verus_solver.cli", "solve",
        str(stitched_file),
        "--out", str(validated_file),
        "--config", str(VERUS_SOLVER_CONFIG),
    ]
    t0 = time.perf_counter()
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True,
            cwd=str(VERUS_SOLVER_REPO), env=env, timeout=600,
        )
        elapsed = time.perf_counter() - t0
        out = r.stdout + r.stderr
        # Parse result JSON from stdout.
        try:
            result = json.loads(r.stdout.strip())
            return bool(result.get("success")), out, elapsed
        except Exception:
            return r.returncode == 0, out, elapsed
    except subprocess.TimeoutExpired:
        elapsed = time.perf_counter() - t0
        return False, "TIMEOUT", elapsed
    except Exception as e:
        elapsed = time.perf_counter() - t0
        return False, str(e), elapsed


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def run_pipeline(bench: str, targets: list[str], verbose: bool = False):
    data = REPO / "data" / bench
    cpp_dir     = data / "cpp"
    prepost_dir = data / "prepost"
    processed_dir = data / "processed"
    stitched_dir  = data / "stitched"
    validated_dir = data / "validated"

    for d in (processed_dir, stitched_dir, validated_dir):
        d.mkdir(parents=True, exist_ok=True)

    if not targets:
        targets = sorted(
            p.stem for p in cpp_dir.glob("*.cpp") if p.stem not in SKIP
        )

    print(f"\n{BLD}Pipeline: {bench}  ({len(targets)} targets){RST}")
    print(f"  verus_solver config: {VERUS_SOLVER_CONFIG}")
    print()

    # Table header
    col = 22
    header = f"{'Target':<25}  {'Preprocess':^{col}}  {'Synthesize':^{col}}  {'verus_solver':^{col}}"
    print(BLD + header + RST)
    print("─" * len(header))

    rows = []
    for target in targets:
        cpp_file     = cpp_dir     / f"{target}.cpp"
        prepost_file = prepost_dir / f"{target}.rs"
        json_file    = processed_dir / f"{target}.json"
        stitched_file = stitched_dir  / f"{target}.rs"
        validated_file = validated_dir / f"{target}.rs"

        if not cpp_file.exists():
            print(f"  {target}: no C++ source, skipping")
            continue
        if not prepost_file.exists():
            print(f"  {target}: no prepost spec, skipping")
            continue

        row = {"target": target}

        # Stage 1: Preprocess
        ok1, out1, t1 = stage_preprocess(cpp_file, json_file)
        row["preprocess"] = (ok1, t1)
        if verbose and not ok1:
            print(f"\n[preprocess {target}]\n{out1}\n")

        # Stage 2: Synthesize + Stitch
        if ok1:
            ok2, out2, t2 = stage_synthesize(json_file, prepost_file, stitched_file)
        else:
            ok2, out2, t2 = None, "", None
        row["synthesize"] = (ok2, t2)
        if verbose and ok2 is False:
            print(f"\n[synthesize {target}]\n{out2}\n")

        # Stage 3: verus_solver
        # Use freshly stitched file if synthesis succeeded, else fall back to
        # pre-existing stitched file (so we can evaluate verus_solver independently).
        src = stitched_file if stitched_file.exists() else None
        if src:
            ok3, out3, t3 = stage_verify(src, validated_file)
        else:
            ok3, out3, t3 = None, "", None
        row["verify"] = (ok3, t3)
        if verbose and ok3 is False:
            print(f"\n[verify {target}]\n{out3}\n")

        rows.append(row)

        # Print row
        def fmt(pair):
            if pair is None or pair[0] is None:
                return f"{'—':^{col}}"
            ok, t = pair
            sym = f"{GRN}PASS{RST}" if ok else f"{RED}FAIL{RST}"
            ts = f"({t:.1f}s)" if t is not None else ""
            return f"{sym} {ts:>8}".ljust(col + 9)  # extra for ANSI

        p = fmt(row["preprocess"])
        s = fmt(row["synthesize"])
        v = fmt(row["verify"])
        print(f"  {target:<23}  {p}  {s}  {v}")

    # Summary
    print()
    n_pre  = sum(1 for r in rows if r["preprocess"][0])
    n_syn  = sum(1 for r in rows if r["synthesize"][0])
    n_ver  = sum(1 for r in rows if r["verify"][0])
    n      = len(rows)
    print(f"{BLD}Summary: preprocess {n_pre}/{n}  synthesize {n_syn}/{n}  verus_solver {n_ver}/{n}{RST}")
    print()


def main():
    ap = argparse.ArgumentParser(description="cpp2rust end-to-end pipeline")
    ap.add_argument("--bench", default="benchmark0", help="benchmark directory under data/")
    ap.add_argument("--targets", nargs="*", default=[], help="specific targets (default: all)")
    ap.add_argument("--verbose", "-v", action="store_true", help="print stage output on failure")
    args = ap.parse_args()
    run_pipeline(args.bench, args.targets, args.verbose)


if __name__ == "__main__":
    main()
