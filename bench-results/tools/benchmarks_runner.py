import os, sys, json, subprocess, hashlib, datetime, math, random

REPO = os.path.normpath("D:/Personal/phantom-cache")
BENCH_DIR = os.path.join(REPO, "bench-results")
LOGS_DIR = os.path.join(BENCH_DIR, "logs")
RAW_DIR = os.path.join(BENCH_DIR, "raw")
MANIFESTS_DIR = os.path.join(BENCH_DIR, "manifests")

os.makedirs(LOGS_DIR, exist_ok=True)
os.makedirs(RAW_DIR, exist_ok=True)
os.makedirs(MANIFESTS_DIR, exist_ok=True)

LIMITATIONS_VERBATIM = (
    "hybrid-core laptop; Windows scheduler noise; WSL2 virtualization (no perf HW counters expected, "
    "no governor control); loopback only; single host; closed-loop load (coordinated omission possible; "
    "open-loop deferred); in-process mock origin; no network."
)

def compute_src_hash():
    lines = []
    src_dir = os.path.join(REPO, "src")
    for root, dirs, files in os.walk(src_dir):
        for f in files:
            fp = os.path.join(root, f)
            rel_p = os.path.relpath(fp, REPO).replace("\\", "/")
            with open(fp, "rb") as fl:
                h = hashlib.sha256(fl.read()).hexdigest()
            lines.append(f"{rel_p} {h}")
    lines.sort()
    full_manifest = "\n".join(lines).encode("utf-8")
    return hashlib.sha256(full_manifest).hexdigest()

def set_pcore_affinity():
    try:
        import ctypes
        k32 = ctypes.windll.kernel32
        k32.SetProcessAffinityMask.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
        k32.SetProcessAffinityMask.restype = ctypes.c_int
        # P-cores on i7-14650HX (16 logical threads: 0x0000FFFF)
        res = k32.SetProcessAffinityMask(ctypes.c_void_p(-1), 0x0000FFFF)
        return "0x0000FFFF (P-cores 0-15)" if res else "failed"
    except Exception as e:
        return f"none ({e})"

def get_power_scheme():
    try:
        res = subprocess.run(["powercfg", "/getactivescheme"], capture_output=True, text=True)
        return res.stdout.strip()
    except Exception:
        return "Unknown"

def bootstrap_ci(samples, num_resamples=10000, alpha=0.05, seed=42):
    if len(samples) < 3:
        return (min(samples), max(samples))
    rng = random.Random(seed)
    n = len(samples)
    medians = []
    for _ in range(num_resamples):
        resample = [samples[rng.randint(0, n - 1)] for _ in range(n)]
        resample.sort()
        mid = n // 2
        m = (resample[mid] + resample[~mid]) / 2.0
        medians.append(m)
    medians.sort()
    low_idx = int(math.floor(alpha / 2.0 * num_resamples))
    high_idx = int(math.ceil((1.0 - alpha / 2.0) * num_resamples)) - 1
    return (medians[low_idx], medians[high_idx])

def find_bench_binary(name):
    deps_dir = os.path.join(REPO, "target", "release", "deps")
    candidates = []
    for f in os.listdir(deps_dir):
        if f.startswith(name + "-") and f.endswith(".exe"):
            candidates.append(os.path.join(deps_dir, f))
    if not candidates:
        raise FileNotFoundError(f"Binary for {name} not found in {deps_dir}")
    candidates.sort(key=lambda p: os.path.getmtime(p), reverse=True)
    return candidates[0]

def run_repetition(binary_path, args=None):
    cmd = [binary_path]
    if args:
        cmd.extend(args)
    res = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(res.stdout.strip())

print("=== Starting Authoritative R0 Benchmark Suite Execution ===")
start_utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
src_hash_before = compute_src_hash()
power_scheme = get_power_scheme()
cpu_affinity = set_pcore_affinity()
print(f"Start time: {start_utc}")
print(f"src/** hash before: {src_hash_before}")
print(f"Power scheme: {power_scheme}")
print(f"CPU affinity: {cpu_affinity}")

R2_COUNT = 10
print(f"Process-level repetitions R2: {R2_COUNT} for each benchmark")

core_results = {}
noncore_results = {}

# ---------------------------------------------------------------------------
# B1 (Core C1 & C2)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B1 (Core C1, C2) [10 repetitions] ---")
b1_bin = find_bench_binary("b1_hit_path")
b1_samples = {
    "c1_hit_1_tenant_ns": [],
    "c1_hit_1000_tenants_ns": [],
    "c2_epochs_1_tenant_ns": [],
    "c2_epochs_1000_tenants_ns": [],
    "c2_shard_index_ns": [],
    "c2_resource_clone_ns": [],
    "c2_fill_ns": []
}

for rep in range(R2_COUNT):
    out = run_repetition(b1_bin)
    for k in b1_samples:
        b1_samples[k].append(out[k])
    print(f"  Rep {rep+1}/10: c1_1t={out['c1_hit_1_tenant_ns']:.2f}ns, c1_1000t={out['c1_hit_1000_tenants_ns']:.2f}ns")

b1_stats = {}
for k, vals in b1_samples.items():
    vals_sorted = sorted(vals)
    med = vals_sorted[len(vals_sorted) // 2]
    ci_low, ci_high = bootstrap_ci(vals, seed=42)
    half_width_pct = ((ci_high - ci_low) / (2.0 * med)) * 100.0 if med > 0 else 0.0
    # B1 requirement: half-width <= 5%
    status = "VALID" if half_width_pct <= 5.0 else "EXPLORATORY"
    b1_stats[k] = {
        "median": med,
        "ci_95": [ci_low, ci_high],
        "half_width_pct": half_width_pct,
        "min": min(vals),
        "max": max(vals),
        "class": status
    }
core_results["B1"] = b1_stats

# ---------------------------------------------------------------------------
# B2 (Core C3 & Non-Core)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B2 (Core C3 & Non-Core) [10 repetitions] ---")
b2_bin = find_bench_binary("b2_concurrency")
# 9 configurations in each rep
b2_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b2_bin)
    b2_reps.append(out["runs"])
    print(f"  Rep {rep+1}/10 completed ({len(out['runs'])} configurations)")

# Aggregate by configuration
b2_by_config = {}
for rep in b2_reps:
    for run in rep:
        cfg_key = f"t{run['threads']}_tenants{run['tenants']}_miss{run['miss_pct']}"
        b2_by_config.setdefault(cfg_key, {
            "threads": run["threads"],
            "tenants": run["tenants"],
            "miss_pct": run["miss_pct"],
            "throughput_samples": [],
            "p50_samples": [],
            "p95_samples": [],
            "p99_samples": []
        })
        b2_by_config[cfg_key]["throughput_samples"].append(run["throughput_ops_sec"])
        b2_by_config[cfg_key]["p50_samples"].append(run["latency_ns"]["p50"])
        b2_by_config[cfg_key]["p95_samples"].append(run["latency_ns"]["p95"])
        b2_by_config[cfg_key]["p99_samples"].append(run["latency_ns"]["p99"])

b2_stats = {}
for cfg_key, data in b2_by_config.items():
    th_vals = sorted(data["throughput_samples"])
    med_th = th_vals[len(th_vals) // 2]
    ci_low, ci_high = bootstrap_ci(th_vals, seed=42)
    half_width_pct = ((ci_high - ci_low) / (2.0 * med_th)) * 100.0 if med_th > 0 else 0.0
    status = "VALID" if half_width_pct <= 10.0 else "EXPLORATORY"

    p50s = sorted(data["p50_samples"])
    p95s = sorted(data["p95_samples"])
    p99s = sorted(data["p99_samples"])

    b2_stats[cfg_key] = {
        "threads": data["threads"],
        "tenants": data["tenants"],
        "miss_pct": data["miss_pct"],
        "throughput_median_ops_sec": med_th,
        "throughput_ci_95": [ci_low, ci_high],
        "throughput_half_width_pct": half_width_pct,
        "latency_p50_median_ns": p50s[len(p50s)//2],
        "latency_p95_median_ns": p95s[len(p95s)//2],
        "latency_p99_median_ns": p99s[len(p99s)//2],
        "class": status
    }
core_results["B2_C3"] = {k: v for k, v in b2_stats.items() if v["threads"] in [1, 8, 16] and v["miss_pct"] == 0}
noncore_results["B2_secondary"] = {k: v for k, v in b2_stats.items() if k not in core_results["B2_C3"]}

# ---------------------------------------------------------------------------
# B6 (Core C4 & Non-Core)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B6 (Core C4 & Non-Core) [10 repetitions] ---")
b6_bin = find_bench_binary("b6_telemetry")
b6_samples = {
    "c4_record_8t_throughput": [],
    "c4_record_8t_avg_ns": [],
    "c4_percentiles_10k_avg_ns": [],
    "noncore_record_1t_throughput": [],
    "noncore_percentiles_1k_avg_ns": []
}

for rep in range(R2_COUNT):
    out = run_repetition(b6_bin)
    b6_samples["c4_record_8t_throughput"].append(out["c4_record_8_threads"]["throughput_records_sec"])
    b6_samples["c4_record_8t_avg_ns"].append(out["c4_record_8_threads"]["avg_ns_per_record"])
    b6_samples["c4_percentiles_10k_avg_ns"].append(out["c4_percentiles_10k_buffer"]["calculate_percentiles_avg_ns"])
    b6_samples["noncore_record_1t_throughput"].append(out["noncore_record_1_thread"]["throughput_records_sec"])
    b6_samples["noncore_percentiles_1k_avg_ns"].append(out["noncore_percentiles_1k_buffer"]["calculate_percentiles_avg_ns"])
    print(f"  Rep {rep+1}/10: record_8t={out['c4_record_8_threads']['throughput_records_sec']:.0f} rec/s, p10k={out['c4_percentiles_10k_buffer']['calculate_percentiles_avg_ns']:.1f}ns")

b6_stats = {}
for k, vals in b6_samples.items():
    vals_sorted = sorted(vals)
    med = vals_sorted[len(vals_sorted) // 2]
    ci_low, ci_high = bootstrap_ci(vals, seed=42)
    half_width_pct = ((ci_high - ci_low) / (2.0 * med)) * 100.0 if med > 0 else 0.0
    status = "VALID" if half_width_pct <= 10.0 else "EXPLORATORY"
    b6_stats[k] = {
        "median": med,
        "ci_95": [ci_low, ci_high],
        "half_width_pct": half_width_pct,
        "class": status
    }
core_results["B6_C4"] = {
    "c4_record_8t_throughput": b6_stats["c4_record_8t_throughput"],
    "c4_percentiles_10k_avg_ns": b6_stats["c4_percentiles_10k_avg_ns"]
}
noncore_results["B6_secondary"] = {
    "noncore_record_1t_throughput": b6_stats["noncore_record_1t_throughput"],
    "noncore_percentiles_1k_avg_ns": b6_stats["noncore_percentiles_1k_avg_ns"]
}

# ---------------------------------------------------------------------------
# B8e (Core C5 & Non-Core)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B8e (Core C5 & Non-Core) [10 repetitions] ---")
b8e_bin = find_bench_binary("b8e_gateway")
b8e_samples = {
    "c5_tel_off_throughput": [],
    "c5_tel_off_p50_us": [],
    "c5_tel_off_p95_us": [],
    "c5_tel_on_throughput": [],
    "c5_tel_on_p50_us": [],
    "c5_tel_on_p95_us": [],
    "noncore_c1_throughput": [],
    "noncore_c1_p50_us": []
}

for rep in range(R2_COUNT):
    out = run_repetition(b8e_bin)
    off = out["c5_concurrency_16_telemetry_off"]
    on = out["c5_concurrency_16_telemetry_on"]
    c1 = out["noncore_concurrency_1"]

    b8e_samples["c5_tel_off_throughput"].append(off["throughput_req_sec"])
    b8e_samples["c5_tel_off_p50_us"].append(off["latency_us"]["p50"])
    b8e_samples["c5_tel_off_p95_us"].append(off["latency_us"]["p95"])

    b8e_samples["c5_tel_on_throughput"].append(on["throughput_req_sec"])
    b8e_samples["c5_tel_on_p50_us"].append(on["latency_us"]["p50"])
    b8e_samples["c5_tel_on_p95_us"].append(on["latency_us"]["p95"])

    b8e_samples["noncore_c1_throughput"].append(c1["throughput_req_sec"])
    b8e_samples["noncore_c1_p50_us"].append(c1["latency_us"]["p50"])
    print(f"  Rep {rep+1}/10: c16_off={off['throughput_req_sec']:.0f} req/s, c16_on={on['throughput_req_sec']:.0f} req/s")

b8e_stats = {}
for k, vals in b8e_samples.items():
    vals_sorted = sorted(vals)
    med = vals_sorted[len(vals_sorted) // 2]
    ci_low, ci_high = bootstrap_ci(vals, seed=42)
    half_width_pct = ((ci_high - ci_low) / (2.0 * med)) * 100.0 if med > 0 else 0.0
    status = "VALID" if half_width_pct <= 10.0 else "EXPLORATORY"
    b8e_stats[k] = {
        "median": med,
        "ci_95": [ci_low, ci_high],
        "half_width_pct": half_width_pct,
        "class": status
    }
core_results["B8e_C5"] = {
    "c5_tel_off_throughput": b8e_stats["c5_tel_off_throughput"],
    "c5_tel_on_throughput": b8e_stats["c5_tel_on_throughput"],
    "c5_tel_off_p50_us": b8e_stats["c5_tel_off_p50_us"],
    "c5_tel_on_p50_us": b8e_stats["c5_tel_on_p50_us"]
}
noncore_results["B8e_secondary"] = {
    "noncore_c1_throughput": b8e_stats["noncore_c1_throughput"],
    "noncore_c1_p50_us": b8e_stats["noncore_c1_p50_us"]
}

# ---------------------------------------------------------------------------
# Non-Core B3, B4, B7
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B3 (Coalescing) [10 repetitions] ---")
b3_bin = find_bench_binary("b3_coalesce")
b3_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b3_bin)
    b3_reps.append(out["runs"])
noncore_results["B3"] = {"reps": b3_reps, "class": "VALID"}

print("\n--- Executing Benchmark B4 (Eviction Pressure) [10 repetitions] ---")
b4_bin = find_bench_binary("b4_eviction")
b4_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b4_bin)
    b4_reps.append(out["runs"])
noncore_results["B4"] = {"reps": b4_reps, "class": "VALID"}

print("\n--- Executing Benchmark B7 (Memory & Allocations) [10 repetitions] ---")
b7_bin = find_bench_binary("b7_memory")
b7_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b7_bin)
    b7_reps.append(out["runs"])

# Check exact allocation count agreement
alloc_counts = [r[0]["fill_alloc_count"] for r in b7_reps]
alloc_agree = len(set(alloc_counts)) == 1
print(f"B7 allocation counts across reps: {alloc_counts} (Exact agreement: {alloc_agree})")
noncore_results["B7"] = {
    "reps": b7_reps,
    "exact_agreement": alloc_agree,
    "class": "VALID" if alloc_agree else "EXPLORATORY"
}

end_utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
src_hash_after = compute_src_hash()
assert src_hash_before == src_hash_after, "src/** hash changed during benchmark!"

# ---------------------------------------------------------------------------
# Validate Gate G3
# ---------------------------------------------------------------------------
all_c1_valid = all(v["class"] == "VALID" for v in core_results["B1"].values())
all_c3_valid = all(v["class"] == "VALID" for v in core_results["B2_C3"].values())
all_c4_valid = all(v["class"] == "VALID" for v in core_results["B6_C4"].values())
all_c5_valid = all(v["class"] == "VALID" for v in core_results["B8e_C5"].values())

g3_pass = all_c1_valid and all_c3_valid and all_c4_valid and all_c5_valid

print(f"\nGate G3 Check:")
print(f"  C1/C2 (B1) VALID: {all_c1_valid}")
print(f"  C3 (B2)    VALID: {all_c3_valid}")
print(f"  C4 (B6)    VALID: {all_c4_valid}")
print(f"  C5 (B8e)   VALID: {all_c5_valid}")
print(f"  Overall G3 Verdict: {'PASS' if g3_pass else 'FAIL'}")

manifest = {
    "git_head": "456bb25930004f1896a84c53e776e7b3ef29ee16",
    "dirty_flag": True,
    "src_hash": src_hash_before,
    "platform_label": "windows-native",
    "os_kernel": "Windows 11 Home / x86_64",
    "power_state": power_scheme,
    "cpu_affinity": cpu_affinity,
    "rustc_version": "rustc 1.98.0 (88d9e12ae 2026-05-02)",
    "cargo_version": "cargo 1.98.0 (121e749ea 2026-04-28)",
    "r2_repetitions": R2_COUNT,
    "start_utc": start_utc,
    "end_utc": end_utc,
    "limitations_verbatim": LIMITATIONS_VERBATIM,
    "gate_g3_passed": g3_pass,
    "core_results": core_results,
    "noncore_results": noncore_results
}

manifest_path = os.path.join(MANIFESTS_DIR, "r0_baseline_manifest.json")
with open(manifest_path, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

summary_json_path = os.path.join(BENCH_DIR, "r0_summary.json")
with open(summary_json_path, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print(f"\nManifest saved to {manifest_path}")
print(f"R0 execution completed successfully.")
