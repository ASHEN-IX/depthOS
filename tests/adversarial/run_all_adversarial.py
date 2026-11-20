#!/usr/bin/env python3
"""Unified runner for Phase 3.8 Adversarial Security Tests.

Executes all independent attack suites:
1. Native prompt integrity, lock/seat races, resurrection attacks (C++ with ASan/UBSan).
2. Injected input misclassification and diagnostic-only invariant (Python + Rust).
3. Production D-Bus read-only allowlist (Python + Rust).
4. Rust coordinator lease concurrency, fencing, quarantine, and crash-cut (Cargo test).
"""

from pathlib import Path
import shlex
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]


def run_step(label, command, cwd=ROOT):
    print(f"\n=======================================================")
    print(f"=== [ADVERSARIAL SUITE] {label}")
    print(f"=======================================================")
    start = time.monotonic()
    res = subprocess.run(command, cwd=cwd, text=True, capture_output=True)
    duration = time.monotonic() - start
    print(res.stdout)
    if res.returncode != 0:
        print(f"FAILED: {label} (exit code {res.returncode})", file=sys.stderr)
        print(res.stderr, file=sys.stderr)
        raise RuntimeError(f"Step '{label}' failed")
    print(f"PASSED: {label} ({duration:.2f}s)")


def main():
    print("Beginning Phase 3.8 Independent Adversarial Test Suite Execution...")
    start_total = time.monotonic()

    # Step 1: C++ Native Authority & Prompt Integrity (ASan + UBSan)
    cflags = shlex.split(subprocess.check_output(["pkg-config", "--cflags", "Qt6Core"], text=True))
    libs = shlex.split(subprocess.check_output(["pkg-config", "--libs", "Qt6Core"], text=True))
    bin_path = ROOT / "target/adversarial_native_authority"
    bin_path.parent.mkdir(parents=True, exist_ok=True)

    compile_cmd = [
        "c++", "-std=c++20", "-fPIC", "-Wall", "-Wextra", "-Werror", "-g",
        "-fsanitize=address,undefined", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer",
        "-DDEPTHOS_AUTHORITY_UNIT_TEST",
        "-I", str(ROOT / "experiments/kwin-authority/native"),
        str(ROOT / "tests/adversarial/test_prompt_integrity_adversarial.cpp"),
        "-o", str(bin_path),
        *cflags, *libs
    ]
    run_step("Compile Native Authority Adversarial Test (ASan + UBSan)", compile_cmd)
    run_step("Execute Native Authority Adversarial Test", [str(bin_path)])

    # Step 2: Injected Input Misclassification
    run_step("Execute Injected Input Classification Attacks", [
        sys.executable, str(ROOT / "tests/adversarial/test_input_classification_adversarial.py")
    ])

    # Step 3: Production D-Bus Read-Only Allowlist
    run_step("Execute Production D-Bus Allowlist Audit", [
        sys.executable, str(ROOT / "tests/adversarial/test_production_allowlist_adversarial.py")
    ])

    # Step 4: Rust Coordinator Lease Adversarial Suite
    run_step("Execute Coordinator Lease Adversarial Suite (Rust)", [
        "cargo", "test", "--test", "adversarial_lease", "--locked", "--offline"
    ])

    # Step 5: Adversarial SQLite Durability, Concurrency & Corruption
    run_step("Execute SQLite Durability & Concurrency Adversarial Tests", [
        sys.executable, str(ROOT / "tests/adversarial/test_sqlite_durability_adversarial.py")
    ])

    # Step 6: Security Contract Claim Adversarial Audit
    run_step("Execute Security Contract Claim Adversarial Audit", [
        sys.executable, str(ROOT / "tests/adversarial/test_contract_claims_adversarial.py")
    ])

    # Step 7: Rust Durable Coordinator Protocol Adversarial Suite
    run_step("Execute Durable Coordinator Protocol Adversarial Suite (Rust)", [
        "cargo", "test", "--test", "adversarial_durable_coordinator", "--locked", "--offline"
    ])

    # Step 8: Phase 3.10 Cross-Process Protocol & Durable Integration Suite (Rust)
    run_step("Execute Cross-Process Protocol & Durable Integration Suite (Rust)", [
        "cargo", "test", "--test", "cross_process_durable", "--locked", "--offline"
    ])

    # Step 9: Phase 3.10.1 Native Authority Adversarial Test (ASan + UBSan)
    bin_path_3_10_1 = ROOT / "target/adversarial_native_authority_3_10_1"
    compile_cmd_3_10_1 = [
        "c++", "-std=c++20", "-fPIC", "-Wall", "-Wextra", "-Werror", "-g",
        "-fsanitize=address,undefined", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer",
        "-DDEPTHOS_AUTHORITY_UNIT_TEST",
        "-I", str(ROOT / "experiments/kwin-authority/native"),
        str(ROOT / "tests/adversarial/test_native_authority_phase_3_10_1.cpp"),
        "-o", str(bin_path_3_10_1),
        *cflags, *libs
    ]
    run_step("Compile Phase 3.10.1 Native Authority Adversarial Test (ASan + UBSan)", compile_cmd_3_10_1)
    run_step("Execute Phase 3.10.1 Native Authority Adversarial Test", [str(bin_path_3_10_1)])

    # Step 10: Phase 3.10.1 IPC & Process Trust Adversarial Test
    run_step("Execute Phase 3.10.1 IPC & Process Trust Adversarial Test", [
        sys.executable, str(ROOT / "tests/adversarial/test_phase_3_10_1_ipc_trust_adversarial.py")
    ])

    # Step 11: Phase 3.10.1 Rust Adversarial Suite
    run_step("Execute Phase 3.10.1 Durable & Reconciliation Suite (Rust)", [
        "cargo", "test", "--test", "adversarial_phase_3_10_1", "--locked", "--offline"
    ])

    # Step 12: Phase 3.10.1 Real Private KWin Cross-Process Integration Runner
    run_step("Execute Phase 3.10.1 Real Private KWin Integration Runner", [
        sys.executable, str(ROOT / "tests/phase-3-native/run_phase_3_10_native.py")
    ])

    total_duration = time.monotonic() - start_total
    print("\n=======================================================")
    print(f"=== ALL PHASE 3.8 / 3.9 / 3.10 / 3.10.1 ADVERSARIAL SUITES PASSED ({total_duration:.2f}s) ===")
    print("=======================================================")


if __name__ == "__main__":
    main()
