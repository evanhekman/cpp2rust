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
    Compile fn_src as a Rust program and run it against all test cases.
    Returns True if all test cases pass, False otherwise.
    Silently discards compilation errors.
    """
    for tc in test_cases:
        inputs = tc["inputs"]
        expected = tc["expected_output"]
        harness = wrap_in_harness(fn_src, fn_name, inputs)
        try:
            actual = _compile_and_run(harness)
        except CompilationError:
            return False
        except subprocess.TimeoutExpired:
            return False
        if actual.strip() != str(expected).strip():
            return False
    return True

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
