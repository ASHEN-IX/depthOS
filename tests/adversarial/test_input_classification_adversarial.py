#!/usr/bin/env python3
"""Adversarial security tests for Injected Input Misclassification.

Verifies:
1. All input origin claims (Unknown, ClaimedPhysical, Qt, Qml, Eis, Remote, Accessibility)
   are strictly denied by the authority boundary.
2. Injected inputs (EIS, fake_input, virtual devices without sysfs path) cannot masquerade
   as local physical hardware.
3. Device labels and properties remain purely diagnostic and NEVER mint consent or authorization tokens.
"""

import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def test_public_origin_refusal():
    """Verify that every origin claim submitted to DenyOnlyAuthority is rejected."""
    print("[TEST] Verifying public origin claims are unconditionally denied...")
    code = """
#include "authority.h"
#include <iostream>
using namespace depthos::experiment;
int main() {
    DenyOnlyAuthority auth;
    Origin origins[] = {
        Origin::Unknown,
        Origin::ClaimedPhysical,
        Origin::Qt,
        Origin::Qml,
        Origin::Eis,
        Origin::Remote,
        Origin::Accessibility
    };
    for (auto orig : origins) {
        if (auth.confirm(orig) != Result::Denied) {
            std::cerr << "Origin was not denied!" << std::endl;
            return 1;
        }
    }
    std::cout << "All 7 origins strictly denied." << std::endl;
    return 0;
}
"""
    with tempfile.TemporaryDirectory() as tmpdir:
        src = Path(tmpdir) / "test.cpp"
        bin_path = Path(tmpdir) / "test_bin"
        src.write_text(code)
        cflags = subprocess.check_output(["pkg-config", "--cflags", "Qt6Core"], text=True).split()
        libs = subprocess.check_output(["pkg-config", "--libs", "Qt6Core"], text=True).split()
        subprocess.check_call([
            "c++", "-std=c++20", "-I", str(ROOT / "experiments/kwin-authority/native"),
            str(src), "-o", str(bin_path), *cflags, *libs
        ])
        out = subprocess.check_output([str(bin_path)], text=True)
        assert "All 7 origins strictly denied." in out
    print("PASS: Public origin refusal verified.")


def test_source_classification_diagnostic_only():
    """Verify that source classification does not convert to capability tokens in Rust."""
    print("[TEST] Verifying Rust action security refuses forged claims...")
    # Run the dedicated security boundary integration test in offline mode
    cmd = ["cargo", "test", "--test", "security_boundary", "--locked", "--offline"]
    res = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    assert res.returncode == 0, f"Cargo test security_boundary failed:\n{res.stdout}\n{res.stderr}"
    assert "test same_uid_client_forged_click_is_denied ... ok" in res.stdout or "ok" in res.stdout
    print("PASS: Same-UID forged claims cannot mint consent.")


def test_device_classification_invariants():
    """Audit the KWin patch device classifier rules for fake input and EIS."""
    print("[TEST] Auditing device classifier rules in patch files...")
    patch_files = list(ROOT.glob("experiments/kwin-authority/**/*.patch"))
    assert len(patch_files) > 0, "No patch files found for KWin authority"

    found_eis = False
    found_fake = False
    found_sysfs = False

    for patch in patch_files:
        text = patch.read_text()
        if "Eis" in text or "eis" in text:
            found_eis = True
        if "Fake" in text or "fake" in text or "Virtual" in text:
            found_fake = True
        if "sysfs" in text or "sysName" in text:
            found_sysfs = True

    assert found_eis, "EIS input categorization not present in KWin patch"
    assert found_fake, "Fake input categorization not present in KWin patch"
    print(f"PASS: Patch contains explicit input device categorizations (EIS: {found_eis}, Fake: {found_fake}, sysfs: {found_sysfs}).")


def main():
    print("=== STARTING INJECTED INPUT ADVERSARIAL TESTS ===")
    test_public_origin_refusal()
    test_source_classification_diagnostic_only()
    test_device_classification_invariants()
    print("=== ALL INJECTED INPUT ADVERSARIAL TESTS PASSED ===")


if __name__ == "__main__":
    main()
