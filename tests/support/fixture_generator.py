#!/usr/bin/env python3
"""Deterministic hierarchy fixture generator for DepthOS Phase 4.5.

Generates reproducible SQLite schema v3 fixtures and synthetic window datasets
for scalability, memory profiling, and reliability qualification.
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/debug/depthos"

SCENARIOS = {
    "small": {"worlds": 4, "workspaces_per_world": 3, "desktops_per_workspace": 3},      # 4 / 12 / 36
    "medium": {"worlds": 20, "workspaces_per_world": 5, "desktops_per_workspace": 3},    # 20 / 100 / 300
    "large": {"worlds": 100, "workspaces_per_world": 5, "desktops_per_workspace": 3},   # 100 / 500 / 1,500
    "stress": {"worlds": 250, "workspaces_per_world": 5, "desktops_per_workspace": 4},  # 250 / 1,250 / 5,000
}

TONES = ["cyan", "lavender", "emerald", "amber", "rose", "precision"]
ARTWORKS = ["matrix", "orbital", "grid", "deep-space"]
ROLES = ["Code", "Review", "Test", "Terminal", "Docs", "Ops"]


def deterministic_uuid(seed_str: str) -> uuid.UUID:
    """Derive deterministic UUIDv4 from a stable seed string."""
    return uuid.uuid5(uuid.NAMESPACE_DNS, f"depthos.phase45.{seed_str}")


def generate_fixture(db_path: Path, scenario_name: str, verbose: bool = False) -> dict:
    """Generate deterministic SQLite v3 database fixture for a given scenario."""
    if scenario_name not in SCENARIOS:
        raise ValueError(f"Unknown scenario '{scenario_name}'. Expected: {list(SCENARIOS.keys())}")

    spec = SCENARIOS[scenario_name]
    num_worlds = spec["worlds"]
    ws_per_world = spec["workspaces_per_world"]
    dt_per_ws = spec["desktops_per_workspace"]

    db_path = Path(db_path).resolve()
    db_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        os.chmod(db_path.parent, 0o700)
    except OSError:
        pass
    if db_path.exists():
        db_path.unlink()

    # 1. Initialize schema v3 using depthos init (without default hierarchy)
    proc = subprocess.run(
        [str(EXE), "--db", str(db_path), "init"],
        capture_output=True, text=True, check=True
    )
    if verbose:
        print(f"[fixture_generator] Initialized {db_path} via depthos init: {proc.stdout.strip()}")

    # 2. Seed deterministically in a single transaction
    t0 = time.monotonic()
    conn = sqlite3.connect(str(db_path))
    conn.execute("PRAGMA foreign_keys = ON;")
    conn.execute("PRAGMA synchronous = NORMAL;")
    conn.execute("BEGIN TRANSACTION;")

    now_ms = int(time.time() * 1000)

    world_rows = []
    workspace_rows = []
    desktop_rows = []

    total_desktops = 0
    total_workspaces = 0

    for w_idx in range(num_worlds):
        w_uuid = deterministic_uuid(f"w-{w_idx}")
        w_id = w_uuid.bytes
        w_name = f"World {w_idx + 1:03d}"
        w_desc = f"Domain context {w_idx + 1} for operations and development"
        w_tone = TONES[w_idx % len(TONES)]
        w_art = ARTWORKS[w_idx % len(ARTWORKS)]
        world_rows.append((w_id, w_name, w_desc, w_idx, now_ms, now_ms, w_tone, w_art, 0, None, 1))

        for ws_idx in range(ws_per_world):
            ws_global_idx = total_workspaces
            total_workspaces += 1
            ws_uuid = deterministic_uuid(f"ws-{w_idx}-{ws_idx}")
            ws_id = ws_uuid.bytes
            ws_name = f"Workspace {w_idx + 1:03d}-{ws_idx + 1:02d}"
            ws_desc = f"Project initiative {ws_idx + 1} in {w_name}"
            ws_root = f"/home/ashenix/Projects/project-{ws_global_idx:04d}"
            ws_tone = TONES[(w_idx + ws_idx) % len(TONES)]
            workspace_rows.append((ws_id, w_id, ws_name, ws_desc, ws_root, ws_idx, now_ms, now_ms, ws_tone, 0, None, 1))

            for dt_idx in range(dt_per_ws):
                dt_global_idx = total_desktops
                total_desktops += 1
                dt_uuid = deterministic_uuid(f"dt-{w_idx}-{ws_idx}-{dt_idx}")
                dt_id = dt_uuid.bytes
                dt_name = f"Desktop {dt_idx + 1:02d}"
                dt_role = ROLES[dt_idx % len(ROLES)]
                dt_tone = TONES[(w_idx + dt_idx) % len(TONES)]
                desktop_rows.append((dt_id, ws_id, dt_name, dt_role, dt_idx, now_ms, now_ms, dt_tone, 0, None, 1))

    conn.executemany(
        """INSERT INTO worlds (id, name, description, sort_order, created_at_ms, updated_at_ms,
                               visual_identity, artwork_identifier, is_archived, archived_at_ms, version)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        world_rows
    )
    conn.executemany(
        """INSERT INTO workspaces (id, world_id, name, description, root_directory, sort_order,
                                  created_at_ms, updated_at_ms, visual_identity, is_archived, archived_at_ms, version)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        workspace_rows
    )
    conn.executemany(
        """INSERT INTO desktops (id, workspace_id, name, role, sort_order, created_at_ms,
                                updated_at_ms, visual_identity, is_archived, archived_at_ms, version)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        desktop_rows
    )

    conn.commit()
    conn.close()
    elapsed = time.monotonic() - t0

    stats = {
        "scenario": scenario_name,
        "path": str(db_path),
        "worlds": num_worlds,
        "workspaces": total_workspaces,
        "desktops": total_desktops,
        "generationTimeSeconds": round(elapsed, 4),
        "fileSizeBytes": db_path.stat().st_size
    }
    if verbose:
        print(f"[fixture_generator] Generated {scenario_name}: {stats}")
    return stats


def generate_synthetic_windows(count: int, desktop_id: str = None) -> list:
    """Generate synthetic observed window records for load benchmarks."""
    windows = []
    base_apps = [
        ("org.kde.kate", "Kate Text Editor"),
        ("org.kde.konsole", "Terminal"),
        ("org.mozilla.firefox", "Firefox Web Browser"),
        ("org.kde.dolphin", "File Manager"),
        ("com.google.Chrome", "Chrome Browser"),
        ("com.visualstudio.code", "VS Code Editor"),
    ]
    for i in range(count):
        app_id, title = base_apps[i % len(base_apps)]
        u = deterministic_uuid(f"win-{i}")
        windows.append({
            "id": str(u),
            "appId": app_id,
            "title": f"{title} #{i}",
            "titleRedacted": False,
            "focused": i == 0,
            "minimized": False,
            "desktopId": desktop_id or str(deterministic_uuid("dt-0-0-0")),
            "assignment": "observed",
            "observedAt": "just now",
        })
    return windows


def main():
    parser = argparse.ArgumentParser(description="DepthOS Phase 4.5 Fixture Generator")
    parser.add_argument("--scenario", choices=list(SCENARIOS.keys()), default="small")
    parser.add_argument("--out", type=Path, default=Path("/tmp/depthos-fixture.sqlite"))
    parser.add_argument("--windows", type=int, default=0)
    args = parser.parse_args()

    stats = generate_fixture(args.out, args.scenario, verbose=True)
    if args.windows > 0:
        wins = generate_synthetic_windows(args.windows)
        print(f"Generated {len(wins)} synthetic window records.")
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
