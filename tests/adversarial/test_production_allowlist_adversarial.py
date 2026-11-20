#!/usr/bin/env python3
"""Adversarial security tests for Production D-Bus Read-Only Allowlist.

Verifies:
1. Production D-Bus interfaces export strictly read-only inspection methods.
2. The exact allowed methods on org.depthos.Controller1 are:
   - get_capabilities
   - get_controller_health
   - get_hierarchy
   - get_context_report
   - get_current_logical_context
3. The exact allowed signals on org.depthos.Controller1 are:
   - invalidated
4. No writable or mutation methods (e.g. MoveDesktop, ExecuteAction, MintConsent, Authorize) are exported.
5. Production capability remains strictly denied.
"""

from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

EXACT_ALLOWED_METHODS = {
    "get_capabilities",
    "get_controller_health",
    "get_hierarchy",
    "get_context_report",
    "get_current_logical_context",
    "get_desktop_shelves",
}

EXACT_ALLOWED_SIGNALS = {
    "invalidated",
}


def test_dbus_read_only_methods():
    """Verify that spatial bridge exports only the exact read-only query methods and signals."""
    print("[TEST] Auditing spatial bridge Rust source for exported D-Bus methods and signals...")
    spatial_mod = ROOT / "src/spatial/mod.rs"
    assert spatial_mod.exists(), f"Missing {spatial_mod}"
    text = spatial_mod.read_text()

    # Find the #[zbus::interface(name = "org.depthos.Controller1")] block
    match = re.search(
        r'#\[zbus::interface\(name = "org\.depthos\.Controller1"\)\]\s*impl\s+Endpoint\s*\{(.*?)\n\}',
        text,
        re.DOTALL
    )
    assert match, "Could not locate #[zbus::interface(name = \"org.depthos.Controller1\")] block in src/spatial/mod.rs"

    block = match.group(1)

    # Separate methods and signals
    signals = set()
    methods = set()

    for item in re.finditer(r"(#\[zbus\(signal\)\]\s*)?async\s+fn\s+([a-zA-Z0-9_]+)\s*\(", block):
        is_signal = bool(item.group(1))
        name = item.group(2)
        if is_signal:
            signals.add(name)
        else:
            methods.add(name)

    print(f"Discovered exported D-Bus methods on org.depthos.Controller1: {sorted(methods)}")
    print(f"Discovered exported D-Bus signals on org.depthos.Controller1: {sorted(signals)}")

    # Verify exact match with allowlist
    assert methods == EXACT_ALLOWED_METHODS, (
        f"Exported methods mismatch! Extra: {methods - EXACT_ALLOWED_METHODS}, "
        f"Missing: {EXACT_ALLOWED_METHODS - methods}"
    )
    assert signals == EXACT_ALLOWED_SIGNALS, (
        f"Exported signals mismatch! Extra: {signals - EXACT_ALLOWED_SIGNALS}, "
        f"Missing: {EXACT_ALLOWED_SIGNALS - signals}"
    )

    forbidden_patterns = [
        r".*move.*",
        r".*execute.*",
        r".*authorize.*",
        r".*consent.*",
        r".*mutate.*",
        r".*write.*",
        r".*delete.*",
        r".*kill.*"
    ]

    for method in methods:
        for pat in forbidden_patterns:
            assert not re.fullmatch(pat, method.lower()), (
                f"Forbidden mutative or authorization method '{method}' exported on production D-Bus!"
            )

    print("PASS: org.depthos.Controller1 exports exactly the 6 approved read-only inspection methods and 1 invalidation signal.")


def test_security_boundary_integration():
    """Run cargo test spatial_bridge and security_boundary to verify D-Bus allowlist."""
    print("[TEST] Running Rust security boundary and spatial bridge integration tests...")
    cmd = [
        "cargo", "test", "--workspace", "--locked", "--offline",
        "--test", "spatial_bridge",
        "--test", "security_boundary"
    ]
    res = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    assert res.returncode == 0, f"Tests failed:\n{res.stdout}\n{res.stderr}"
    print("PASS: Production D-Bus read-only allowlist verified by integration tests.")


def main():
    print("=== STARTING PRODUCTION ALLOWLIST ADVERSARIAL TESTS ===")
    test_dbus_read_only_methods()
    test_security_boundary_integration()
    print("=== ALL PRODUCTION ALLOWLIST ADVERSARIAL TESTS PASSED ===")


if __name__ == "__main__":
    main()
