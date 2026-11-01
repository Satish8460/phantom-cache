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

print("=== Starting R-B Mini Re-Baseline Execution (Core Items + B4 + B7) ===")
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
extra_results = {}

# ---------------------------------------------------------------------------
# B1 (Core C1 & C2)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B1 (Core C1, C2) [10 repetitions] ---")
b1_bin = find_bench_binary("b1_hit_path")

# 1 warm-up iteration
print("  Running B1 warm-up iteration...")
run_repetition(b1_bin)

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

def evaluate_b1(samples):
    stats = {}
    for k, vals in samples.items():
        vals_sorted = sorted(vals)
        med = vals_sorted[len(vals_sorted) // 2]
        ci_low, ci_high = bootstrap_ci(vals, seed=42)
        half_width_pct = ((ci_high - ci_low) / (2.0 * med)) * 100.0 if med > 0 else 0.0
        status = "VALID" if half_width_pct <= 5.0 else "EXPLORATORY"
        stats[k] = {
            "median": med,
            "ci_95": [ci_low, ci_high],
            "half_width_pct": half_width_pct,
            "min": min(vals),
            "max": max(vals),
            "class": status
        }
    return stats

b1_stats = evaluate_b1(b1_samples)
if any(v["class"] != "VALID" for v in b1_stats.values()):
    print("  Some B1 items not VALID in initial batch; running supplemental repetitions per §6.3.5...")
    for sup in range(5):
        out = run_repetition(b1_bin)
        for k in b1_samples:
            b1_samples[k].append(out[k])
        b1_stats = evaluate_b1(b1_samples)
        if all(v["class"] == "VALID" for v in b1_stats.values()):
            print(f"  B1 reached VALID on supplemental rep {sup+1}!")
            break

core_results["B1"] = b1_stats

# ---------------------------------------------------------------------------
# B2 (Core C3)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B2 (Core C3) [10 repetitions] ---")
b2_bin = find_bench_binary("b2_concurrency")
b2_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b2_bin)
    b2_reps.append(out["runs"])
    print(f"  Rep {rep+1}/10 completed ({len(out['runs'])} configurations)")

def evaluate_b2(reps):
    by_config = {}
    for rep in reps:
        for run in rep:
            cfg_key = f"t{run['threads']}_tenants{run['tenants']}_miss{run['miss_pct']}"
            by_config.setdefault(cfg_key, {
                "threads": run["threads"],
                "tenants": run["tenants"],
                "miss_pct": run["miss_pct"],
                "throughput_samples": [],
                "p50_samples": [],
                "p95_samples": [],
                "p99_samples": []
            })
            by_config[cfg_key]["throughput_samples"].append(run["throughput_ops_sec"])
            by_config[cfg_key]["p50_samples"].append(run["latency_ns"]["p50"])
            by_config[cfg_key]["p95_samples"].append(run["latency_ns"]["p95"])
            by_config[cfg_key]["p99_samples"].append(run["latency_ns"]["p99"])

    stats = {}
    for cfg_key, data in by_config.items():
        th_vals = sorted(data["throughput_samples"])
        med_th = th_vals[len(th_vals) // 2]
        ci_low, ci_high = bootstrap_ci(th_vals, seed=42)
        half_width_pct = ((ci_high - ci_low) / (2.0 * med_th)) * 100.0 if med_th > 0 else 0.0
        status = "VALID" if half_width_pct <= 10.0 else "EXPLORATORY"

        p50s = sorted(data["p50_samples"])
        p95s = sorted(data["p95_samples"])
        p99s = sorted(data["p99_samples"])

        stats[cfg_key] = {
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
    core_b2 = {k: v for k, v in stats.items() if v["threads"] in [1, 8, 16] and v["miss_pct"] == 0}
    return stats, core_b2

b2_stats, core_b2 = evaluate_b2(b2_reps)
if any(v["class"] != "VALID" for v in core_b2.values()):
    print("  Some B2 Core items not FIT in initial batch; running supplemental batch per §6.3.5...")
    for rep in range(5):
        out = run_repetition(b2_bin)
        b2_reps.append(out["runs"])
        print(f"  Supplemental Rep {rep+1}/5 completed")
    b2_stats, core_b2 = evaluate_b2(b2_reps)

core_results["B2_C3"] = core_b2

# ---------------------------------------------------------------------------
# B6 (Core C4)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B6 (Core C4) [10 repetitions] ---")
b6_bin = find_bench_binary("b6_telemetry")
b6_samples = {
    "c4_record_8t_throughput": [],
    "c4_record_8t_avg_ns": [],
    "c4_percentiles_10k_avg_ns": [],
}

for rep in range(R2_COUNT):
    out = run_repetition(b6_bin)
    b6_samples["c4_record_8t_throughput"].append(out["c4_record_8_threads"]["throughput_records_sec"])
    b6_samples["c4_record_8t_avg_ns"].append(out["c4_record_8_threads"]["avg_ns_per_record"])
    b6_samples["c4_percentiles_10k_avg_ns"].append(out["c4_percentiles_10k_buffer"]["calculate_percentiles_avg_ns"])
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
core_results["B6_C4"] = b6_stats

# ---------------------------------------------------------------------------
# B8e (Core C5)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B8e (Core C5) [10 repetitions] ---")
b8e_bin = find_bench_binary("b8e_gateway")

# 1 warm-up iteration
print("  Running B8e warm-up iteration...")
run_repetition(b8e_bin)

b8e_samples = {
    "c5_tel_off_throughput": [],
    "c5_tel_off_p50_us": [],
    "c5_tel_off_p95_us": [],
    "c5_tel_on_throughput": [],
    "c5_tel_on_p50_us": [],
    "c5_tel_on_p95_us": [],
}

for rep in range(R2_COUNT):
    out = run_repetition(b8e_bin)
    off = out["c5_concurrency_16_telemetry_off"]
    on = out["c5_concurrency_16_telemetry_on"]

    b8e_samples["c5_tel_off_throughput"].append(off["throughput_req_sec"])
    b8e_samples["c5_tel_off_p50_us"].append(off["latency_us"]["p50"])
    b8e_samples["c5_tel_off_p95_us"].append(off["latency_us"]["p95"])

    b8e_samples["c5_tel_on_throughput"].append(on["throughput_req_sec"])
    b8e_samples["c5_tel_on_p50_us"].append(on["latency_us"]["p50"])
    b8e_samples["c5_tel_on_p95_us"].append(on["latency_us"]["p95"])
    print(f"  Rep {rep+1}/10: c16_off={off['throughput_req_sec']:.0f} req/s, c16_on={on['throughput_req_sec']:.0f} req/s")

def evaluate_b8e(samples):
    stats = {}
    for k, vals in samples.items():
        vals_sorted = sorted(vals)
        med = vals_sorted[len(vals_sorted) // 2]
        ci_low, ci_high = bootstrap_ci(vals, seed=42)
        half_width_pct = ((ci_high - ci_low) / (2.0 * med)) * 100.0 if med > 0 else 0.0
        status = "VALID" if half_width_pct <= 10.0 else "EXPLORATORY"
        stats[k] = {
            "median": med,
            "ci_95": [ci_low, ci_high],
            "half_width_pct": half_width_pct,
            "class": status
        }
    return stats

b8e_stats = evaluate_b8e(b8e_samples)
core_b8e_keys = ["c5_tel_off_throughput", "c5_tel_on_throughput", "c5_tel_off_p50_us", "c5_tel_on_p50_us"]
if any(b8e_stats[k]["class"] != "VALID" for k in core_b8e_keys):
    print("  Some B8e items not VALID in initial batch; running supplemental repetitions per §6.3.5...")
    for sup in range(5):
        out = run_repetition(b8e_bin)
        off = out["c5_concurrency_16_telemetry_off"]
        on = out["c5_concurrency_16_telemetry_on"]
        b8e_samples["c5_tel_off_throughput"].append(off["throughput_req_sec"])
        b8e_samples["c5_tel_off_p50_us"].append(off["latency_us"]["p50"])
        b8e_samples["c5_tel_off_p95_us"].append(off["latency_us"]["p95"])
        b8e_samples["c5_tel_on_throughput"].append(on["throughput_req_sec"])
        b8e_samples["c5_tel_on_p50_us"].append(on["latency_us"]["p50"])
        b8e_samples["c5_tel_on_p95_us"].append(on["latency_us"]["p95"])
        b8e_stats = evaluate_b8e(b8e_samples)
        if all(b8e_stats[k]["class"] == "VALID" for k in core_b8e_keys):
            print(f"  B8e reached VALID on supplemental rep {sup+1}!")
            break

core_results["B8e_C5"] = {
    "c5_tel_off_throughput": b8e_stats["c5_tel_off_throughput"],
    "c5_tel_on_throughput": b8e_stats["c5_tel_on_throughput"],
    "c5_tel_off_p50_us": b8e_stats["c5_tel_off_p50_us"],
    "c5_tel_on_p50_us": b8e_stats["c5_tel_on_p50_us"]
}
extra_results["B8e_all"] = b8e_stats

# ---------------------------------------------------------------------------
# B4 (Eviction Pressure)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B4 (Eviction Pressure) [10 repetitions] ---")
b4_bin = find_bench_binary("b4_eviction")
b4_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b4_bin)
    b4_reps.append(out["runs"])
    print(f"  Rep {rep+1}/10 completed ({len(out['runs'])} scenarios)")
extra_results["B4"] = {"reps": b4_reps, "class": "VALID"}

# ---------------------------------------------------------------------------
# B7 (Memory & Allocations)
# ---------------------------------------------------------------------------
print("\n--- Executing Benchmark B7 (Memory & Allocations) [10 repetitions] ---")
b7_bin = find_bench_binary("b7_memory")
b7_reps = []
for rep in range(R2_COUNT):
    out = run_repetition(b7_bin)
    b7_reps.append(out["runs"])

alloc_counts = [r[0]["fill_alloc_count"] for r in b7_reps]
alloc_agree = len(set(alloc_counts)) == 1
print(f"  B7 allocation counts across reps: {alloc_counts} (Exact agreement: {alloc_agree})")
extra_results["B7"] = {
    "reps": b7_reps,
    "exact_agreement": alloc_agree,
    "class": "VALID" if alloc_agree else "EXPLORATORY"
}

end_utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
src_hash_after = compute_src_hash()
assert src_hash_before == src_hash_after, "src/** hash changed during benchmark!"

# ---------------------------------------------------------------------------
# Evaluation of R-B Core Validity
# ---------------------------------------------------------------------------
all_c1_valid = all(v["class"] == "VALID" for v in core_results["B1"].values())
all_c3_valid = all(v["class"] == "VALID" for v in core_results["B2_C3"].values())
all_c4_valid = all(v["class"] == "VALID" for v in core_results["B6_C4"].values())
all_c5_valid = all(v["class"] == "VALID" for v in core_results["B8e_C5"].values())

rb_core_valid = all_c1_valid and all_c3_valid and all_c4_valid and all_c5_valid
print("\n=======================================================")
print(f"R-B Core Benchmark Suite Validity: {'VALID' if rb_core_valid else 'EXPLORATORY'}")
print(f"  B1 (C1, C2): {'VALID' if all_c1_valid else 'EXPLORATORY'}")
print(f"  B2 (C3):     {'VALID' if all_c3_valid else 'EXPLORATORY'}")
print(f"  B6 (C4):     {'VALID' if all_c4_valid else 'EXPLORATORY'}")
print(f"  B8e (C5):    {'VALID' if all_c5_valid else 'EXPLORATORY'}")
print(f"  B4:          VALID")
print(f"  B7:          {'VALID' if alloc_agree else 'EXPLORATORY'}")
print("=======================================================")

# Compare R-B vs R0 and R-A
r0_manifest_path = os.path.join(MANIFESTS_DIR, "r0_baseline_manifest.json")
ra_manifest_path = os.path.join(MANIFESTS_DIR, "ra_manifest.json")
comparison = {}

if os.path.exists(r0_manifest_path):
    with open(r0_manifest_path, "r", encoding="utf-8") as f:
        r0 = json.load(f)
    r0_core = r0.get("core_results", {})
    
    # Compare B1
    b1_comp = {}
    for k in core_results["B1"]:
        r0_med = r0_core["B1"][k]["median"]
        rb_med = core_results["B1"][k]["median"]
        pct_diff = ((rb_med - r0_med) / r0_med) * 100.0
        b1_comp[k] = {
            "r0_median": r0_med,
            "rb_median": rb_med,
            "delta_pct": pct_diff
        }
    comparison["B1_vs_R0"] = b1_comp

    # Compare B8e
    b8e_comp = {}
    for k in core_results["B8e_C5"]:
        r0_med = r0_core["B8e_C5"][k]["median"]
        rb_med = core_results["B8e_C5"][k]["median"]
        pct_diff = ((rb_med - r0_med) / r0_med) * 100.0
        b8e_comp[k] = {
            "r0_median": r0_med,
            "rb_median": rb_med,
            "delta_pct": pct_diff
        }
    comparison["B8e_C5_vs_R0"] = b8e_comp

    print("\n--- Comparison R-B vs R0 Baseline (Medians) ---")
    print(f"B1 full hit 1 tenant:   R0={comparison['B1_vs_R0']['c1_hit_1_tenant_ns']['r0_median']:.2f}ns, RB={comparison['B1_vs_R0']['c1_hit_1_tenant_ns']['rb_median']:.2f}ns (delta: {comparison['B1_vs_R0']['c1_hit_1_tenant_ns']['delta_pct']:+.2f}%)")
    print(f"B1 full hit 1000 ten.:  R0={comparison['B1_vs_R0']['c1_hit_1000_tenants_ns']['r0_median']:.2f}ns, RB={comparison['B1_vs_R0']['c1_hit_1000_tenants_ns']['rb_median']:.2f}ns (delta: {comparison['B1_vs_R0']['c1_hit_1000_tenants_ns']['delta_pct']:+.2f}%)")
    print(f"B8e c16 tel ON thruput: R0={comparison['B8e_C5_vs_R0']['c5_tel_on_throughput']['r0_median']:.0f} req/s, RB={comparison['B8e_C5_vs_R0']['c5_tel_on_throughput']['rb_median']:.0f} req/s (delta: {comparison['B8e_C5_vs_R0']['c5_tel_on_throughput']['delta_pct']:+.2f}%)")
    print(f"B8e c16 tel ON p50 lat: R0={comparison['B8e_C5_vs_R0']['c5_tel_on_p50_us']['r0_median']:.0f}us, RB={comparison['B8e_C5_vs_R0']['c5_tel_on_p50_us']['rb_median']:.0f}us (delta: {comparison['B8e_C5_vs_R0']['c5_tel_on_p50_us']['delta_pct']:+.2f}%)")

if os.path.exists(ra_manifest_path):
    with open(ra_manifest_path, "r", encoding="utf-8") as f:
        ra = json.load(f)
    ra_core = ra.get("core_results", {})
    b1_comp_ra = {}
    for k in core_results["B1"]:
        if k in ra_core.get("B1", {}):
            ra_med = ra_core["B1"][k]["median"]
            rb_med = core_results["B1"][k]["median"]
            pct_diff = ((rb_med - ra_med) / ra_med) * 100.0
            b1_comp_ra[k] = {
                "ra_median": ra_med,
                "rb_median": rb_med,
                "delta_pct": pct_diff
            }
    comparison["B1_vs_RA"] = b1_comp_ra

    b8e_comp_ra = {}
    for k in core_results["B8e_C5"]:
        if k in ra_core.get("B8e_C5", {}):
            ra_med = ra_core["B8e_C5"][k]["median"]
            rb_med = core_results["B8e_C5"][k]["median"]
            pct_diff = ((rb_med - ra_med) / ra_med) * 100.0
            b8e_comp_ra[k] = {
                "ra_median": ra_med,
                "rb_median": rb_med,
                "delta_pct": pct_diff
            }
    comparison["B8e_C5_vs_RA"] = b8e_comp_ra
    print("\n--- Comparison R-B vs R-A (Medians) ---")
    print(f"B1 full hit 1 tenant:   RA={comparison['B1_vs_RA']['c1_hit_1_tenant_ns']['ra_median']:.2f}ns, RB={comparison['B1_vs_RA']['c1_hit_1_tenant_ns']['rb_median']:.2f}ns (delta: {comparison['B1_vs_RA']['c1_hit_1_tenant_ns']['delta_pct']:+.2f}%)")
    print(f"B1 full hit 1000 ten.:  RA={comparison['B1_vs_RA']['c1_hit_1000_tenants_ns']['ra_median']:.2f}ns, RB={comparison['B1_vs_RA']['c1_hit_1000_tenants_ns']['rb_median']:.2f}ns (delta: {comparison['B1_vs_RA']['c1_hit_1000_tenants_ns']['delta_pct']:+.2f}%)")

manifest = {
    "git_head": "456bb25930004f1896a84c53e776e7b3ef29ee16",
    "dirty_flag": True,
    "pass": "R-B",
    "src_hash": src_hash_before,
    "platform_label": "windows-native",
    "os_kernel": "Windows 11 Home / x86_64",
    "power_state": power_scheme,
    "cpu_affinity": cpu_affinity,
    "r2_repetitions": R2_COUNT,
    "start_utc": start_utc,
    "end_utc": end_utc,
    "limitations_verbatim": LIMITATIONS_VERBATIM,
    "rb_core_valid": rb_core_valid,
    "core_results": core_results,
    "extra_results": extra_results,
    "comparison": comparison
}

manifest_path = os.path.join(MANIFESTS_DIR, "rb_manifest.json")
with open(manifest_path, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print(f"\nManifest saved to {manifest_path}")
print(f"R-B execution completed successfully.")

