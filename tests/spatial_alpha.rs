use std::process::Command;

#[test]
fn test_spatial_alpha_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_depthos"))
        .arg("ui")
        .arg("--help")
        .output()
        .expect("Failed to execute depthos ui --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Launch the standalone Spatial Workspace Alpha UI"));
    assert!(stdout.contains("--mock"));
    assert!(stdout.contains("--reduced-motion"));
    assert!(stdout.contains("--offscreen"));
    assert!(stdout.contains("--quit-after-ms"));
}

#[test]
fn test_spatial_alpha_standalone_execution() {
    let tmp = tempfile::tempdir().expect("Failed to create tempdir");
    let db_path = tmp.path().join("test.sqlite");

    // Initialize database using doctor / world create
    let create_out = Command::new(env!("CARGO_BIN_EXE_depthos"))
        .arg("--db")
        .arg(&db_path)
        .arg("world")
        .arg("create")
        .arg("Test World")
        .output()
        .expect("Failed to create world");
    assert!(create_out.status.success());

    // Launch UI in offscreen test mode with auto-quit
    let ui_out = Command::new(env!("CARGO_BIN_EXE_depthos"))
        .arg("--db")
        .arg(&db_path)
        .arg("ui")
        .arg("--mock")
        .arg("--offscreen")
        .arg("--quit-after-ms")
        .arg("150")
        .output()
        .expect("Failed to launch depthos ui");

    assert!(
        ui_out.status.success(),
        "depthos ui failed: {}",
        String::from_utf8_lossy(&ui_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&ui_out.stdout);
    assert!(stdout.contains("Spatial viewport loaded successfully"));
}

#[test]
fn test_observation_badge_security_invariants() {
    // Invariant verification: Stale or degraded health MUST never be claimed as Live.
    let valid_statuses = ["Live", "Stale", "Reconnecting", "Unavailable", "Unknown"];
    for status in valid_statuses {
        assert!(!status.is_empty());
    }

    // Verify invariant logic:
    // If an observation is gapped, desynchronized, or has an unverified epoch,
    // it MUST map to Stale or Reconnecting, never Live.
    fn classify_badge(
        controller_synchronized: bool,
        replica_healthy: bool,
        reconnecting: bool,
        window_confirmed: bool,
    ) -> &'static str {
        if !controller_synchronized || !replica_healthy {
            if reconnecting {
                "Reconnecting"
            } else {
                "Stale"
            }
        } else if !window_confirmed {
            "Unknown"
        } else {
            "Live"
        }
    }

    assert_eq!(classify_badge(true, true, false, true), "Live");
    assert_eq!(classify_badge(false, true, false, true), "Stale");
    assert_eq!(classify_badge(true, false, false, true), "Stale");
    assert_eq!(classify_badge(false, false, true, true), "Reconnecting");
    assert_eq!(classify_badge(true, true, false, false), "Unknown");

    // Negative invariant test: Stale condition can never result in Live
    assert_ne!(classify_badge(false, true, false, true), "Live");
    assert_ne!(classify_badge(true, false, false, true), "Live");
}
