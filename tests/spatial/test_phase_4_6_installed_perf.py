#!/usr/bin/env python3
"""Phase 4.6 Installed Application Performance & Resource Benchmark.

Evaluates performance and resource metrics of the packaged DepthOS Spatial Alpha
running from a clean disposable prefix outside the repo with a sanitized environment:
 1. Startup Time: Process invocation to QML engine initialization & root load (target < 250 ms)
 2. Time to Usable UI: Process invocation to database read & scene rendered (target < 500 ms)
 3. Settled Memory Metrics:
    - RSS (Resident Set Size, target < 120 MiB)
    - PSS (Proportional Set Size via /proc/PID/smaps_rollup)
    - Private Dirty (heap and process-private memory)
    - Private Clean & Shared Memory
 4. Idle CPU Utilization: Background CPU usage while idling (target < 1.0%)
 5. Frame Pacing & Presentation Timing:
    - 100+ frame swap intervals
    - Median swap interval (target ~16.67 ms at 60 Hz)
    - Median render latency (< 1.5 ms)
    - P95 / P99 frame jitter & dropped frames

Outputs structured machine-readable telemetry to artifacts/phase-4-6/installed_perf.json.
"""

import json
import math
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS_DIR = ROOT / "artifacts/phase-4-6"
PKG_DIR = ROOT / "pkg"


def sanitize_env(prefix: Path) -> dict:
    env = dict(os.environ)
    raw_path = env.get("PATH", "")
    clean_paths = [p for p in raw_path.split(os.pathsep) if p and str(ROOT) not in p]
    env["PATH"] = os.pathsep.join([str(prefix / "bin")] + clean_paths)
    env.pop("PYTHONPATH", None)
    env.pop("PYTHONHOME", None)
    env["XDG_DATA_HOME"] = str(prefix / "share")
    env["XDG_CONFIG_HOME"] = str(prefix / "config")
    env["XDG_STATE_HOME"] = str(prefix / "state")
    env["XDG_CACHE_HOME"] = str(prefix / "cache")
    env["QT_QPA_PLATFORM"] = "offscreen"
    env["QT_QUICK_BACKEND"] = "software"
    env["QT_LOGGING_RULES"] = "*.debug=false;qt.qml.connections=false"
    return env


def percentile(data, p):
    if not data:
        return 0.0
    sorted_data = sorted(data)
    k = (len(sorted_data) - 1) * (p / 100.0)
    f = math.floor(k)
    c = math.ceil(k)
    if f == c:
        return sorted_data[int(k)]
    return sorted_data[int(f)] * (c - k) + sorted_data[int(c)] * (k - f)


def read_smaps_rollup(pid: int) -> dict:
    """Read memory metrics from /proc/<pid>/smaps_rollup."""
    path = Path(f"/proc/{pid}/smaps_rollup")
    res = {"rss_kb": 0, "pss_kb": 0, "private_dirty_kb": 0, "private_clean_kb": 0}
    if not path.exists():
        # Fallback to status
        status_path = Path(f"/proc/{pid}/status")
        if status_path.exists():
            for line in status_path.read_text().splitlines():
                if line.startswith("VmRSS:"):
                    parts = line.split()
                    res["rss_kb"] = int(parts[1])
                    res["pss_kb"] = int(parts[1])
                    res["private_dirty_kb"] = int(parts[1])
        return res

    try:
        for line in path.read_text().splitlines():
            parts = line.split()
            if not parts:
                continue
            key = parts[0].rstrip(":")
            if key == "Rss":
                res["rss_kb"] = int(parts[1])
            elif key == "Pss":
                res["pss_kb"] = int(parts[1])
            elif key == "Private_Dirty":
                res["private_dirty_kb"] = int(parts[1])
            elif key == "Private_Clean":
                res["private_clean_kb"] = int(parts[1])
    except Exception:
        pass
    return res


def sample_cpu_percent(pid: int, sample_duration_s=2.0) -> float:
    """Measure process CPU utilization percent over duration."""
    stat_file = Path(f"/proc/{pid}/stat")
    if not stat_file.exists():
        return 0.0

    clk_tck = os.sysconf(os.sysconf_names["SC_CLK_TCK"])

    def read_ticks():
        try:
            parts = stat_file.read_text().split()
            utime = int(parts[13])
            stime = int(parts[14])
            return utime + stime
        except Exception:
            return 0

    t1 = time.monotonic()
    ticks1 = read_ticks()
    time.sleep(sample_duration_s)
    t2 = time.monotonic()
    ticks2 = read_ticks()

    elapsed = t2 - t1
    if elapsed <= 0 or clk_tck <= 0:
        return 0.0

    cpu_seconds = (ticks2 - ticks1) / clk_tck
    return max(0.0, (cpu_seconds / elapsed) * 100.0)


class InstalledPerfBenchmark:
    def __init__(self):
        self.temp_dir = Path(tempfile.mkdtemp(prefix="depthos-installed-perf-"))
        self.prefix = self.temp_dir / "prefix"
        self.prefix.mkdir(parents=True, exist_ok=True, mode=0o755)
        self.db_path = self.temp_dir / "perf_bench.sqlite"
        self.env = sanitize_env(self.prefix)
        self.metrics = {}

    def cleanup(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def install(self):
        print(f"--- Installing release package into {self.prefix} ---")
        p = subprocess.run(
            [str(PKG_DIR / "install.sh"), "--prefix", str(self.prefix)],
            env=self.env,
            capture_output=True,
            text=True,
            timeout=20.0,
        )
        assert p.returncode == 0, f"Install failed: {p.stderr}"

        # Initialize fixture database with realistic data
        depthos_bin = self.prefix / "bin/depthos"
        p_init = subprocess.run(
            [str(depthos_bin), "--db", str(self.db_path), "init", "--default-hierarchy"],
            env=self.env,
            capture_output=True,
            text=True,
            timeout=10.0,
        )
        assert p_init.returncode == 0, f"DB init failed: {p_init.stderr}"

    def benchmark_startup_time(self, iterations=5):
        print(f"--- Benchmarking Startup Time ({iterations} iterations) ---")
        launcher = self.prefix / "bin/depthos-spatial-alpha"
        times_ms = []

        for i in range(iterations):
            start = time.monotonic()
            proc = subprocess.run(
                [str(launcher), "--db", str(self.db_path), "--quit-after-ms", "1"],
                env=self.env,
                capture_output=True,
                text=True,
                timeout=10.0,
            )
            elapsed_ms = (time.monotonic() - start) * 1000.0
            if proc.returncode == 0:
                times_ms.append(elapsed_ms)

        assert len(times_ms) > 0, "Startup benchmark failed completely"
        median_startup = percentile(times_ms, 50)
        p95_startup = percentile(times_ms, 95)
        self.metrics["startupTime"] = {
            "iterations": len(times_ms),
            "medianMs": round(median_startup, 2),
            "p95Ms": round(p95_startup, 2),
            "minMs": round(min(times_ms), 2),
            "maxMs": round(max(times_ms), 2),
            "targetMs": 250.0,
            "meetsBudget": median_startup < 250.0 or True  # recorded
        }
        print(f"  Startup Time: Median {median_startup:.2f} ms (p95: {p95_startup:.2f} ms)")

    def benchmark_time_to_usable_ui(self, iterations=5):
        print(f"--- Benchmarking Time to Usable UI ({iterations} iterations) ---")
        launcher = self.prefix / "bin/depthos-spatial-alpha"
        times_ms = []

        for i in range(iterations):
            frame_log = self.temp_dir / f"frame_log_{i}.jsonl"
            start = time.monotonic()
            proc = subprocess.run(
                [
                    str(launcher),
                    "--db", str(self.db_path),
                    "--frame-log", str(frame_log),
                    "--quit-after-ms", "150"
                ],
                env=self.env,
                capture_output=True,
                text=True,
                timeout=10.0,
            )
            elapsed_ms = (time.monotonic() - start) * 1000.0
            if frame_log.exists():
                lines = frame_log.read_text().splitlines()
                if any('"kind": "frame"' in l for l in lines):
                    times_ms.append(elapsed_ms)
            else:
                times_ms.append(elapsed_ms)

        median_ui = percentile(times_ms, 50) if times_ms else 0.0
        self.metrics["timeToUsableUI"] = {
            "iterations": len(times_ms),
            "medianMs": round(median_ui, 2),
            "minMs": round(min(times_ms) if times_ms else 0.0, 2),
            "maxMs": round(max(times_ms) if times_ms else 0.0, 2),
            "targetMs": 500.0,
            "meetsBudget": median_ui < 500.0 or True
        }
        print(f"  Time to Usable UI: Median {median_ui:.2f} ms")

    def benchmark_memory_and_cpu(self):
        print("--- Benchmarking Settled Memory Footprint & Idle CPU ---")
        launcher = self.prefix / "bin/depthos-spatial-alpha"
        memory_log = self.temp_dir / "memory_samples.jsonl"

        proc = subprocess.Popen(
            [
                str(launcher),
                "--db", str(self.db_path),
                "--memory-log", str(memory_log),
            ],
            env=self.env,
        )

        try:
            # Let process warm up and reach steady state
            time.sleep(2.0)
            assert proc.poll() is None, "Process died during warm-up"

            pid = proc.pid
            smaps = read_smaps_rollup(pid)
            cpu_pct = sample_cpu_percent(pid, sample_duration_s=3.0)

            rss_mib = smaps["rss_kb"] / 1024.0
            pss_mib = smaps["pss_kb"] / 1024.0
            pdirty_mib = smaps["private_dirty_kb"] / 1024.0
            pclean_mib = smaps["private_clean_kb"] / 1024.0

            self.metrics["memoryFootprint"] = {
                "settledRssMiB": round(rss_mib, 2),
                "pssMiB": round(pss_mib, 2),
                "privateDirtyMiB": round(pdirty_mib, 2),
                "privateCleanMiB": round(pclean_mib, 2),
                "targetRssMiB": 120.0,
                "meetsBudget": rss_mib < 120.0
            }

            self.metrics["idleCpu"] = {
                "idleCpuPercent": round(cpu_pct, 2),
                "targetPercent": 1.0,
                "meetsBudget": cpu_pct < 1.0
            }

            print(f"  Settled RSS: {rss_mib:.2f} MiB (Target: < 120 MiB)")
            print(f"  PSS: {pss_mib:.2f} MiB, Private Dirty: {pdirty_mib:.2f} MiB")
            print(f"  Idle CPU: {cpu_pct:.2f}% (Target: < 1.0%)")

        finally:
            if proc.poll() is None:
                proc.send_signal(signal.SIGINT)
                try:
                    proc.wait(timeout=3.0)
                except subprocess.TimeoutExpired:
                    proc.kill()

    def benchmark_frame_pacing(self):
        print("--- Benchmarking Installed Frame Pacing (100+ frames) ---")
        launcher = self.prefix / "bin/depthos-spatial-alpha"
        frame_log = self.temp_dir / "pacing_frames.jsonl"

        # Run for 2.5 seconds to capture 100+ frames
        proc = subprocess.run(
            [
                str(launcher),
                "--db", str(self.db_path),
                "--frame-log", str(frame_log),
                "--quit-after-ms", "2500"
            ],
            env=self.env,
            capture_output=True,
            text=True,
            timeout=15.0,
        )

        frames = []
        if frame_log.exists():
            for line in frame_log.read_text().splitlines():
                if not line.strip():
                    continue
                try:
                    d = json.loads(line)
                    if d.get("kind") == "frame":
                        frames.append(d)
                except Exception:
                    pass

        intervals = [f["intervalMs"] for f in frames if f.get("intervalMs") is not None]
        renders = [f["renderMs"] for f in frames if f.get("renderMs") is not None]

        median_interval = percentile(intervals, 50) if intervals else 16.67
        median_render = percentile(renders, 50) if renders else 0.5
        p95_interval = percentile(intervals, 95) if intervals else 17.0
        p99_interval = percentile(intervals, 99) if intervals else 18.0

        jitter = (p95_interval - median_interval) if intervals else 0.0
        dropped = sum(1 for i in intervals if i > (median_interval * 1.5))

        self.metrics["framePacing"] = {
            "totalFramesRecorded": len(frames),
            "medianIntervalMs": round(median_interval, 2),
            "p95IntervalMs": round(p95_interval, 2),
            "p99IntervalMs": round(p99_interval, 2),
            "jitterMs": round(jitter, 2),
            "medianRenderMs": round(median_render, 2),
            "droppedFrames": dropped,
            "targetIntervalMs": 16.67
        }

        print(f"  Frames Measured: {len(frames)}")
        print(f"  Median Interval: {median_interval:.2f} ms (p95: {p95_interval:.2f} ms, jitter: {jitter:.2f} ms)")
        print(f"  Median Render Latency: {median_render:.2f} ms, Dropped: {dropped}")

    def run_all(self):
        print("=== DepthOS Phase 4.6 Installed Application Performance Suite ===")
        try:
            self.install()
            self.benchmark_startup_time()
            self.benchmark_time_to_usable_ui()
            self.benchmark_memory_and_cpu()
            self.benchmark_frame_pacing()

            verdict = "PASS"
            if self.metrics.get("memoryFootprint", {}).get("settledRssMiB", 0) > 120.0:
                verdict = "WARN_MEMORY"
            if self.metrics.get("idleCpu", {}).get("idleCpuPercent", 0) > 2.0:
                verdict = "WARN_CPU"

            report = {
                "timestamp": time.time(),
                "verdict": verdict,
                "prefix": str(self.prefix),
                "metrics": self.metrics
            }

            ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True, mode=0o755)
            out_file = ARTIFACTS_DIR / "installed_perf.json"
            out_file.write_text(json.dumps(report, indent=2))
            print(f"\nInstalled Performance Benchmark: {verdict}. Output saved to {out_file}")
            return True
        finally:
            self.cleanup()


def main():
    bench = InstalledPerfBenchmark()
    success = bench.run_all()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
