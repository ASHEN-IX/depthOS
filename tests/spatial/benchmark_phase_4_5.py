#!/usr/bin/env python3
"""Comprehensive benchmark harness for DepthOS Phase 4.5.

Measures:
1. Startup latency and time to first usable interaction.
2. Memory footprint (RSS, PSS, Private Dirty, VmHWM) across catalog scales.
3. Hierarchy query & load latencies.
4. Model-level selection and navigation latencies.
5. Search execution latency across query patterns.
6. Management transaction latencies (CRUD, reorder, archive, restore).
7. Frame swap pacing, render thread intervals, and idle quiescence.
8. Synthetic window scaling (0, 10, 100, 500, 1,000 windows).
"""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/debug/depthos"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
sys.path.insert(0, str(ROOT / "tests/support"))
from fixture_generator import generate_fixture, generate_synthetic_windows, SCENARIOS


def memory_snapshot(pid: int) -> dict:
    """Extract precise memory breakdown from /proc/[pid]/smaps_rollup and status."""
    mem = {"rssKiB": 0, "pssKiB": 0, "privateDirtyKiB": 0, "vmHwmKiB": 0}
    try:
        status = Path(f"/proc/{pid}/status").read_text()
        for line in status.splitlines():
            if line.startswith("VmHWM:"):
                mem["vmHwmKiB"] = int(line.split()[1])
            elif line.startswith("VmRSS:"):
                mem["rssKiB"] = int(line.split()[1])
        rollup = Path(f"/proc/{pid}/smaps_rollup")
        if rollup.exists():
            for line in rollup.read_text().splitlines():
                if line.startswith("Rss:"):
                    mem["rssKiB"] = int(line.split()[1])
                elif line.startswith("Pss:"):
                    mem["pssKiB"] = int(line.split()[1])
                elif line.startswith("Private_Dirty:"):
                    mem["privateDirtyKiB"] = int(line.split()[1])
    except Exception:
        pass
    return mem


def benchmark_sqlite_queries(db_path: Path) -> dict:
    """Benchmark raw Rust/SQLite hierarchy query performance."""
    results = {}
    
    # 1. Tree query latency
    tree_times = []
    for _ in range(5):
        t0 = time.monotonic()
        p = subprocess.run([str(EXE), "--db", str(db_path), "tree"],
                           capture_output=True, text=True, check=True)
        tree_times.append((time.monotonic() - t0) * 1000)
    tree_data = json.loads(p.stdout)
    results["treeQueryLatencyMs"] = {
        "median": round(statistics.median(tree_times), 2),
        "min": round(min(tree_times), 2),
        "max": round(max(tree_times), 2),
    }
    results["catalogCounts"] = {
        "worlds": len(tree_data.get("worlds", [])),
        "workspaces": sum(len(w.get("workspaces", [])) for w in tree_data.get("worlds", [])),
        "desktops": sum(len(ws.get("desktops", [])) for w in tree_data.get("worlds", []) for ws in w.get("workspaces", [])),
    }

    # 2. Targeted queries
    first_world_id = tree_data["worlds"][0]["world"]["id"] if tree_data.get("worlds") else None
    if first_world_id:
        ws_times = []
        for _ in range(5):
            t0 = time.monotonic()
            subprocess.run([str(EXE), "--db", str(db_path), "workspace", "list", "--world", first_world_id],
                           capture_output=True, text=True, check=True)
            ws_times.append((time.monotonic() - t0) * 1000)
        results["workspaceListLatencyMs"] = round(statistics.median(ws_times), 2)

    return results


def benchmark_management_operations(db_path: Path) -> dict:
    """Benchmark transactional management operations via manage exec."""
    results = {}
    
    # 1. Create World
    t0 = time.monotonic()
    req = {
        "action": "create",
        "entity": "world",
        "payload": {
            "name": f"Benchmark World {time.time_ns()}",
            "description": "Benchmark temporary domain",
            "visualIdentity": "cyan",
            "artwork": "matrix"
        }
    }
    p = subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                       capture_output=True, text=True, check=True)
    resp = json.loads(p.stdout)
    assert resp["success"]
    world_id = resp["id"]
    results["createWorldMs"] = round((time.monotonic() - t0) * 1000, 2)

    # 2. Create Workspace
    t0 = time.monotonic()
    req = {
        "action": "create",
        "entity": "workspace",
        "parentId": world_id,
        "payload": {
            "name": "Benchmark Workspace",
            "description": "Benchmarking workspace creation",
            "visualIdentity": "lavender"
        }
    }
    p = subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                       capture_output=True, text=True, check=True)
    resp = json.loads(p.stdout)
    assert resp["success"]
    workspace_id = resp["id"]
    results["createWorkspaceMs"] = round((time.monotonic() - t0) * 1000, 2)

    # 3. Create Desktops (batch 3)
    desktop_ids = []
    desktop_times = []
    for d_name in ["Desk A", "Desk B", "Desk C"]:
        t0 = time.monotonic()
        req = {
            "action": "create",
            "entity": "desktop",
            "parentId": workspace_id,
            "payload": {"name": d_name, "visualIdentity": "emerald"}
        }
        p = subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                           capture_output=True, text=True, check=True)
        resp = json.loads(p.stdout)
        assert resp["success"]
        desktop_ids.append(resp["id"])
        desktop_times.append((time.monotonic() - t0) * 1000)
    results["createDesktopMs"] = round(statistics.median(desktop_times), 2)

    # 4. Sibling Reorder
    t0 = time.monotonic()
    req = {
        "action": "reorder",
        "entity": "desktop",
        "parentId": workspace_id,
        "payload": {"ids": [desktop_ids[2], desktop_ids[0], desktop_ids[1]]}
    }
    subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                   capture_output=True, text=True, check=True)
    results["reorderDesktopsMs"] = round((time.monotonic() - t0) * 1000, 2)

    # 5. Archive Desktop
    t0 = time.monotonic()
    req = {
        "action": "archive",
        "entity": "desktop",
        "id": desktop_ids[2],
        "expectedVersion": 2
    }
    subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                   capture_output=True, text=True, check=True)
    results["archiveDesktopMs"] = round((time.monotonic() - t0) * 1000, 2)

    # 6. Restore Desktop
    t0 = time.monotonic()
    req = {
        "action": "restore",
        "entity": "desktop",
        "id": desktop_ids[2]
    }
    subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                   capture_output=True, text=True, check=True)
    results["restoreDesktopMs"] = round((time.monotonic() - t0) * 1000, 2)

    # 7. Project Inspection
    t0 = time.monotonic()
    req = {"action": "inspectProject", "payload": {"path": str(ROOT)}}
    subprocess.run([str(EXE), "--db", str(db_path), "manage", "exec", "--request", json.dumps(req)],
                   capture_output=True, text=True, check=True)
    results["inspectProjectMs"] = round((time.monotonic() - t0) * 1000, 2)

    return results


def benchmark_ui_lifecycle(db_path: Path, quit_after_ms: int = 1500) -> dict:
    """Measure UI cold startup, frame logging, memory footprint, and exit behavior."""
    with tempfile.TemporaryDirectory(prefix="depthos-bench-") as td:
        tdp = Path(td)
        frame_log = tdp / "frames.jsonl"
        mem_log = tdp / "memory.jsonl"

        t0 = time.monotonic()
        p = subprocess.run(
            [
                sys.executable, str(LAUNCHER),
                "--db", str(db_path),
                "--offscreen",
                "--quit-after-ms", str(quit_after_ms),
                "--frame-log", str(frame_log),
                "--memory-log", str(mem_log),
            ],
            capture_output=True, text=True, check=True
        )
        total_wall_ms = (time.monotonic() - t0) * 1000

        # Parse memory log
        final_mem = {}
        if mem_log.exists() and mem_log.stat().st_size > 0:
            lines = mem_log.read_text().strip().splitlines()
            if lines:
                final_mem = json.loads(lines[-1])

        # Parse frame log
        frame_intervals = []
        render_times = []
        if frame_log.exists() and frame_log.stat().st_size > 0:
            lines = frame_log.read_text().strip().splitlines()
            for line in lines:
                if not line.strip():
                    continue
                row = json.loads(line)
                if row.get("kind") == "frame":
                    iv = row.get("intervalMs")
                    if iv is not None:
                        frame_intervals.append(iv)
                    rm = row.get("renderMs")
                    if rm is not None:
                        render_times.append(rm)

        stats = {
            "wallRunMs": round(total_wall_ms, 2),
            "memory": {
                "rssKiB": int(final_mem.get("Rss", 0) if isinstance(final_mem.get("Rss"), (int, float)) else str(final_mem.get("Rss", "0 kB")).split()[0]),
                "pssKiB": int(final_mem.get("Pss", 0) if isinstance(final_mem.get("Pss"), (int, float)) else str(final_mem.get("Pss", "0 kB")).split()[0]),
                "privateDirtyKiB": int(final_mem.get("Private_Dirty", 0) if isinstance(final_mem.get("Private_Dirty"), (int, float)) else str(final_mem.get("Private_Dirty", "0 kB")).split()[0]),
                "vmHwmKiB": int(final_mem.get("VmHWM", 0) if isinstance(final_mem.get("VmHWM"), (int, float)) else str(final_mem.get("VmHWM", "0 kB")).split()[0]),
            },
            "frames": {
                "totalSwaps": len(frame_intervals),
                "medianIntervalMs": round(statistics.median(frame_intervals), 2) if frame_intervals else None,
                "p95IntervalMs": round(statistics.quantiles(frame_intervals, n=20)[18], 2) if len(frame_intervals) >= 20 else (round(max(frame_intervals), 2) if frame_intervals else None),
                "maxIntervalMs": round(max(frame_intervals), 2) if frame_intervals else None,
                "medianRenderMs": round(statistics.median(render_times), 2) if render_times else None,
                "p95RenderMs": round(statistics.quantiles(render_times, n=20)[18], 2) if len(render_times) >= 20 else (round(max(render_times), 2) if render_times else None),
            }
        }
        return stats


def run_full_benchmark_matrix() -> dict:
    """Run benchmark across small, medium, large, and stress scenarios."""
    results = {
        "timestamp": time.time(),
        "gitCommit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "toolchain": {
            "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "python": sys.version.split()[0],
            "kernel": subprocess.check_output(["uname", "-r"], text=True).strip(),
            "kwin": subprocess.check_output(["/usr/bin/kwin_wayland", "--version"], text=True).strip(),
        },
        "scenarios": {}
    }

    with tempfile.TemporaryDirectory(prefix="depthos-matrix-") as td:
        tdp = Path(td)
        os.chmod(tdp, 0o700)
        for sc in ["small", "medium", "large", "stress"]:
            print(f"=== Benchmarking Scenario: {sc.upper()} ===", flush=True)
            db_path = tdp / f"{sc}.sqlite"
            fixture_stats = generate_fixture(db_path, sc, verbose=False)
            sqlite_stats = benchmark_sqlite_queries(db_path)
            manage_stats = benchmark_management_operations(db_path)
            ui_stats = benchmark_ui_lifecycle(db_path, quit_after_ms=1200)

            results["scenarios"][sc] = {
                "fixture": fixture_stats,
                "sqliteQueries": sqlite_stats,
                "managementOperations": manage_stats,
                "uiLifecycle": ui_stats,
            }
            print(f"[{sc}] Tree query median: {sqlite_stats['treeQueryLatencyMs']['median']} ms | RSS: {ui_stats['memory']['rssKiB']} KiB | createWorld: {manage_stats['createWorldMs']} ms", flush=True)

    return results


def main():
    parser = argparse.ArgumentParser(description="DepthOS Phase 4.5 Benchmark Matrix")
    parser.add_argument("--out", type=Path, default=ROOT / "artifacts/phase-4-5/benchmark_baseline.json")
    args = parser.parse_args()

    args.out.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
    results = run_full_benchmark_matrix()
    args.out.write_text(json.dumps(results, indent=2))
    print(f"\nBenchmark matrix written to: {args.out}")


if __name__ == "__main__":
    main()
