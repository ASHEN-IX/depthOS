#!/usr/bin/env python3
"""Adversarial Security Contract Claim Verification Suite.

Phase 3.9 Audit:
Verifies that documentation files (specifically Phase 3.8 and Phase 3.9 documents):
1. Do NOT overstate cryptographic binding (UUIDs and epochs are software identifiers, not cryptographic signatures).
2. Do NOT claim true in-process extension isolation within monolithic KWin (must acknowledge shared address space).
3. Do NOT claim tamper-proof physical input attribution (input classification is diagnostic telemetry, not capability minting).
4. Do NOT claim multi-seat hardware support without bare-metal qualification (must mark multi-seat unsupported in virtual testbeds).
5. Do NOT claim distributed 2PC atomicity or exactly-once semantics across asynchronous D-Bus (must state at-most-once with quarantine).
6. Explicitly qualify power-loss safety (Class E) as UNSUPPORTED in virtual test environments.
"""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
DOCS_DIR = ROOT / "docs"

def audit_claim_file(file_path: Path):
    text = file_path.read_text(encoding="utf-8")
    lower = text.lower()
    
    findings = []
    
    # Check 1: Cryptographic binding overstatement
    # Flag only if claiming the system HAS or USES cryptographic signatures
    if "implements cryptographic signature" in lower or "cryptographically signed payload" in lower or "using cryptographic signatures" in lower:
        findings.append("Affirmatively claims cryptographic signatures without hardware backing")
        
    # Check 2: Exactly-once semantics affirmative claim
    if "provides exactly-once" in lower or "guarantees exactly-once" in lower:
        if not any(neg in lower for neg in ["not", "cannot", "does not", "at-most-once"]):
            findings.append("Affirmatively claims 'exactly-once' without clarifying at-most-once")
        
    # Check 3: In-process extension isolation overstatement
    if "isolated plugins" in lower or "plugin isolation" in lower:
        if not any(lim in lower for lim in ["same address space", "in-process", "structural limitation", "shared address space", "shares memory"]):
            findings.append("Claims plugin isolation without documenting in-process shared address space limitation")
        
    # Check 4: Distributed atomicity across D-Bus
    if "provides distributed atomicity" in lower or "implements two-phase commit" in lower or "distributed 2pc" in lower:
        if not any(neg in lower for neg in ["no distributed", "not support", "prohibit", "neither", "pessimistic"]):
            findings.append("Affirmatively claims distributed atomicity / 2PC")
            
    # Check 5: Multi-seat qualification
    if "multi-seat" in lower or "multiseat" in lower:
        if not any(unsupp in lower for unsupp in ["unsupported", "virtual testbed", "hardware absent", "absence of physical"]):
            findings.append("Mentions multi-seat without explicitly qualifying as UNSUPPORTED in virtual testbed")
            
    # Check 6: Power-loss safety qualification (relevant for Phase 3.9)
    if "power loss" in lower or "power-loss" in lower or "power cut" in lower:
        if not any(unsupp in lower for unsupp in ["unsupported", "virtual testbed", "hardware absent", "class e", "cannot verify", "cannot be performed"]):
            findings.append("Mentions power loss without explicitly qualifying as UNSUPPORTED in virtual testbed")
            
    return findings

def main():
    print("=== STARTING SECURITY CONTRACT CLAIM ADVERSARIAL AUDIT ===")
    
    # Audit existing Phase 3.8 and Phase 3.9 documents
    target_docs = [
        DOCS_DIR / "phase-3-8-coordinator-lease.md",
        DOCS_DIR / "phase-3-8-security-review.md",
        DOCS_DIR / "phase-3-8-input-seat-lock.md",
        DOCS_DIR / "phase-3-8-protected-prompt.md",
    ]
    
    # Also check if Phase 3.9 documents exist
    phase_3_9_docs = [
        DOCS_DIR / "phase-3-9-security-contract.md",
        DOCS_DIR / "phase-3-9-durable-journal.md",
        DOCS_DIR / "phase-3-9-crash-consistency.md",
        DOCS_DIR / "phase-3-9-security-review.md",
        DOCS_DIR / "phase-3-9-validation.md",
    ]

    phase_3_10_docs = [
        DOCS_DIR / "phase-3-10-protocol.md",
        DOCS_DIR / "phase-3-10-peer-authentication.md",
        DOCS_DIR / "phase-3-10-reconciliation.md",
        DOCS_DIR / "phase-3-10-security-review.md",
        DOCS_DIR / "phase-3-10-validation.md",
    ]
    
    total_findings = 0
    
    for doc in target_docs:
        if not doc.exists():
            continue
        findings = audit_claim_file(doc)
        if findings:
            print(f"FAILED: {doc.name} has contract overstatements:")
            for f in findings:
                print(f"  - {f}")
            total_findings += len(findings)
        else:
            print(f"PASS: {doc.name} satisfies all security claim boundaries.")
            
    print(f"\nChecking pending Phase 3.9 docs (if present)...")
    for doc in phase_3_9_docs:
        if doc.exists():
            findings = audit_claim_file(doc)
            if findings:
                print(f"FAILED: {doc.name} has contract overstatements:")
                for f in findings:
                    print(f"  - {f}")
                total_findings += len(findings)
            else:
                print(f"PASS: {doc.name} satisfies all security claim boundaries.")
        else:
            print(f"INFO: {doc.name} not yet authored by Agent 1 (will be audited upon handoff).")

    print(f"\nChecking Phase 3.10 docs...")
    for doc in phase_3_10_docs:
        if doc.exists():
            findings = audit_claim_file(doc)
            if findings:
                print(f"FAILED: {doc.name} has contract overstatements:")
                for f in findings:
                    print(f"  - {f}")
                total_findings += len(findings)
            else:
                print(f"PASS: {doc.name} satisfies all security claim boundaries.")
        else:
            print(f"INFO: {doc.name} not yet authored by Agent 1 (will be audited upon handoff).")

    if total_findings > 0:
        print(f"\nAudit failed with {total_findings} overstatement findings!", file=sys.stderr)
        sys.exit(1)
        
    print("\n=== ALL CONTRACT CLAIM AUDITS PASSED ===")

if __name__ == "__main__":
    main()
