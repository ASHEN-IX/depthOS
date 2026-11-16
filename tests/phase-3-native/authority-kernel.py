#!/usr/bin/env python3
"""Build/run isolated deny-only and synthetic C++ kernels; never loads KWin code."""
from pathlib import Path
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def main():
    flags = shlex.split(subprocess.check_output(
        ["pkg-config", "--cflags", "--libs", "Qt6Core"], text=True))
    env = {"PATH": "/usr/bin", "LANG": "C.UTF-8"}
    with tempfile.TemporaryDirectory(prefix="depthos-authority-kernel-") as directory:
        for synthetic in (False, True):
            binary = Path(directory) / ("synthetic" if synthetic else "deny-only")
            command = ["c++", "-std=c++20", "-Wall", "-Wextra", "-Werror", "-g",
                       "-I", str(ROOT / "experiments/kwin-authority/native"),
                       str(ROOT / "tests/phase-3-native/authority-kernel.cpp"),
                       "-o", str(binary), *flags]
            if synthetic:
                command.insert(1, "-DDEPTHOS_AUTHORITY_UNIT_TEST")
            subprocess.run(command, check=True, env=env)
            subprocess.run([str(binary)], check=True, env=env)
            if not synthetic:
                symbols = subprocess.check_output(["nm", "-C", str(binary)], text=True, env=env)
                if "depthos::experiment::synthetic" in symbols:
                    raise RuntimeError("synthetic kernel present in deny-only binary")
                print("PASS deny-only binary contains no synthetic namespace", flush=True)
    print("PASS disposable binaries removed; no bus or display environment supplied", flush=True)


if __name__ == "__main__":
    main()
