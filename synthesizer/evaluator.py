from __future__ import annotations
import os
import subprocess
import tempfile
from typing import List, Dict, Any
from .codegen import wrap_in_harness

COMPILE_TIMEOUT = 10
RUN_TIMEOUT = 5


class CompilationError(Exception):
    pass


def test_candidate(
    fn_src: str,
    fn_name: str,
    test_cases: List[Dict[str, Any]],
) -> bool:
    """
    Compile fn_src once, run it once, and check all test case outputs.
    Returns True if all outputs match. Silently discards compilation errors.
    """
    harness = wrap_in_harness(fn_src, fn_name, test_cases)
    try:
        stdout = _compile_and_run(harness)
    except (CompilationError, subprocess.TimeoutExpired):
        return False

    lines = stdout.splitlines()
    if len(lines) != len(test_cases):
        return False

    return all(
        line.strip() == str(tc["expected_output"]).strip()
        for line, tc in zip(lines, test_cases)
    )


def _compile_and_run(src: str) -> str:
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "candidate.rs")
        bin_path = os.path.join(tmpdir, "candidate")
        with open(src_path, "w") as f:
            f.write(src)

        compile_result = subprocess.run(
            ["rustc", src_path, "-o", bin_path, "--edition=2021"],
            capture_output=True,
            text=True,
            timeout=COMPILE_TIMEOUT,
        )
        if compile_result.returncode != 0:
            raise CompilationError(compile_result.stderr)

        run_result = subprocess.run(
            [bin_path],
            capture_output=True,
            text=True,
            timeout=RUN_TIMEOUT,
        )
        if run_result.returncode != 0:
            raise CompilationError(run_result.stderr)

        return run_result.stdout
