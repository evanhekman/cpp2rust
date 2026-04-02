import subprocess
import argparse
from pathlib import Path
import re

def run_validator(input_path: str, output_path: str, phase_uniform: bool = True, is_baseline: bool = False) -> tuple[int, bool]:
    cmd = ["python", "main.py", "--input", input_path, "--output", output_path]

    if phase_uniform:
        cmd.append("--phase-uniform")
    if is_baseline:
        cmd.append("--is-baseline")

    result = subprocess.run(cmd, cwd="verus-proof-synthesis/autoverus")

    success = False
    try:
        output = Path(output_path).read_text()
        success = bool(re.search(r"// Score: \(\d+, 0\)", output))
    except (FileNotFoundError, OSError):
        pass

    return result.returncode, success

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--phase-uniform", action="store_true", default=True)
    parser.add_argument("--is-baseline", action="store_true", default=False)
    args = parser.parse_args()

    input_path = Path(args.input).resolve()
    output_path = Path(args.output).resolve()

    return_code, success = run_validator(input_path, output_path, args.phase_uniform, args.is_baseline)
    print(f"Return code: {return_code}")
    print(f"Success: {success}")