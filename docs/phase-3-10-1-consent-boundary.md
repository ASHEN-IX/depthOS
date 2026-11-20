# Phase 3.10.1 Consent Token & Transport Boundary Specification

## 1. Architectural Token Taxonomy

DepthOS enforces strict architectural isolation between four distinct security tokens and capabilities. No token may substitute for, escalate to, or mint any other token.

```
+-----------------------------------------------------------------------------------------+
|                                1. Launcher Bootstrap Token                              |
|  - Scope: Unix Domain Socket transport authentication only                              |
|  - Holder: Designated coordinator and compositor launcher processes                     |
|  - Authority: Grants right to perform protocol handshake; NO action or window authority |
+-----------------------------------------------------------------------------------------+
                                             |
                                             v
+-----------------------------------------------------------------------------------------+
|                                  2. Coordinator Lease                                   |
|  - Scope: Specific linear protocol attempt for one window resource                      |
|  - Holder: Rust Durable Coordinator Session                                             |
|  - Authority: Enforces mutual exclusion, intent WAL logging; cannot mutate compositor    |
+-----------------------------------------------------------------------------------------+
                                             |
                                             v
+-----------------------------------------------------------------------------------------+
|                           3. Internal KWin Synthetic Token                              |
|  - Scope: Test fixture counting sink inside private authority experiment                |
|  - Holder: Integration test harness & private authority server                          |
|  - Authority: Increments atomic counter (count_); NEVER calls real native window setters|
+-----------------------------------------------------------------------------------------+
                                             |
                                             v
+-----------------------------------------------------------------------------------------+
|                               4. Human Physical Consent                                 |
|  - Scope: Real production window placement mutation                                     |
|  - Holder: Trusted user via out-of-band, physically verified interaction boundary        |
|  - Status: CURRENTLY UNSUPPORTED / NOT IMPLEMENTED (All production paths DENY-ONLY)     |
+-----------------------------------------------------------------------------------------+
```

---

## 2. Invariants & Security Boundaries

### Invariant 1: Wire Messages Cannot Mint Internal Authority
Receiving an IPC message—even over an authenticated Unix socket with valid peer credentials—does not convey consent.
- The `SyntheticAttemptMessage` carries `fixture_authority_token`, but the authority server validates this token strictly as a test-only authorization to invoke `kernel.syntheticConfirm()`.
- The authority server never exposes any IPC endpoint or method that mutates real desktop or window coordinates (`KWin::Window::move`, `setDesktops`, etc.).

### Invariant 2: Input-Origin Claims Are Diagnostic Evidence Only
Under Wayland and KWin, input events may arrive from physical hardware (`libinput`), synthetic devices (`fakeinput`), remote desktop portals, or accessibility buses (`at-spi`).
- Origin labels (e.g. `Origin::ClaimedPhysical`, `Origin::Qt`, `Origin::Eis`, `Origin::Remote`) are **diagnostic evidence only**.
- Public or client-supplied input origin claims are **never** accepted as proof of genuine human intent.
- As verified in `test_native_authority_phase_3_10_1.cpp` (Test 1.1) and `authority-kernel.cpp`, all public origin claims return `Result::Denied` unconditionally.

### Invariant 3: Production Binaries Never Expose Positive Synthetic Authority
The synthetic counting endpoint exists exclusively under the private compile-time flag `DEPTHOS_AUTHORITY_UNIT_TEST`:
- In standard and production builds, `DenyOnlyAuthority` is the sole compiled authority implementation.
- `DenyOnlyAuthority::confirm()` returns `Result::Denied` unconditionally for all inputs.
- No positive consent tokens can be minted or verified in production builds.

---

## 3. Threat Analysis & Mitigations

| Threat | Attack Vector | Enforced Mitigation |
| :--- | :--- | :--- |
| **Same-UID Peer Spoofing** | Rogue user process on same machine connects to private socket | Rejected at handshake: must supply launcher-issued 128-bit unguessable `bootstrap_token`. |
| **Lease Escalation** | Coordinator sends `SyntheticAttempt` without acquiring reservation | Rejected: authority server requires matching prepared state, generation, and active proposal. |
| **Replay Attack** | Coordinator retransmits previously acknowledged `SyntheticAttempt` | Rejected: server verifies single-use; second attempt returns `Acknowledge(Rejected)` and count remains 1. |
| **Forged Input Injection** | Attacker injects virtual keystroke via EIS or fake input device | Denied: synthetic endpoint rejects all synthetic/injected input devices for production operations. |
| **Cross-Session Resurrect** | Reusing a token across coordinator or compositor restarts | Denied: coordinator incarnation and compositor incarnation are strictly fenced in all wire headers. |
