import time
import json
import statistics
import urllib.request
import concurrent.futures
import subprocess

RUST_URL = "http://127.0.0.1:4567"
JVM_URL = "http://127.0.0.1:4568"

def get_docker_stats():
    res = subprocess.run(
        ["docker", "stats", "--no-stream", "--format", "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}"],
        capture_output=True, text=True
    )
    stats = {}
    for line in res.stdout.strip().split("\n"):
        parts = line.split("\t")
        if len(parts) >= 4:
            stats[parts[0]] = {
                "cpu": parts[1],
                "mem": parts[2],
                "mem_perc": parts[3]
            }
    return stats

def get_image_sizes():
    res = subprocess.run(
        ["docker", "images", "--format", "{{.Repository}}:{{.Tag}}\t{{.Size}}"],
        capture_output=True, text=True
    )
    sizes = {}
    for line in res.stdout.strip().split("\n"):
        parts = line.split("\t")
        if len(parts) == 2:
            sizes[parts[0]] = parts[1]
    return sizes

def bench_request(url, path, method="GET", payload=None, headers=None):
    if headers is None:
        headers = {}
    if payload:
        data = payload.encode() if isinstance(payload, str) else json.dumps(payload).encode()
        headers["Content-Type"] = "application/json"
    else:
        data = None

    req = urllib.request.Request(f"{url}{path}", data=data, headers=headers, method=method)
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            content = resp.read()
            status = resp.status
    except Exception as e:
        status = 500
        content = b""
    t1 = time.perf_counter()
    return (t1 - t0) * 1000.0, status, len(content)

def run_latency_benchmark(name, path, method="GET", payload=None, n=100):
    print(f"\n--- Running Benchmark: {name} (N={n}) ---")
    
    # Warmup
    for _ in range(5):
        bench_request(RUST_URL, path, method, payload)
        bench_request(JVM_URL, path, method, payload)

    # Rust
    rust_latencies = []
    for _ in range(n):
        lat, status, _ = bench_request(RUST_URL, path, method, payload)
        if status == 200:
            rust_latencies.append(lat)

    # JVM
    jvm_latencies = []
    for _ in range(n):
        lat, status, _ = bench_request(JVM_URL, path, method, payload)
        if status == 200:
            jvm_latencies.append(lat)

    rust_latencies.sort()
    jvm_latencies.sort()

    def stats(l):
        if not l:
            return {"min": 0, "avg": 0, "p50": 0, "p95": 0, "p99": 0, "max": 0}
        return {
            "min": round(min(l), 2),
            "avg": round(statistics.mean(l), 2),
            "p50": round(statistics.median(l), 2),
            "p95": round(l[int(len(l) * 0.95)], 2),
            "p99": round(l[int(len(l) * 0.99)], 2),
            "max": round(max(l), 2),
        }

    r_stats = stats(rust_latencies)
    j_stats = stats(jvm_latencies)

    print(f"Rust ({len(rust_latencies)}/{n}): min={r_stats['min']}ms, avg={r_stats['avg']}ms, p50={r_stats['p50']}ms, p95={r_stats['p95']}ms")
    print(f"JVM  ({len(jvm_latencies)}/{n}): min={j_stats['min']}ms, avg={j_stats['avg']}ms, p50={j_stats['p50']}ms, p95={j_stats['p95']}ms")

    return {
        "test": name,
        "rust": r_stats,
        "jvm": j_stats,
        "speedup_avg": round(j_stats["avg"] / r_stats["avg"], 2) if r_stats["avg"] > 0 else 0,
        "speedup_p50": round(j_stats["p50"] / r_stats["p50"], 2) if r_stats["p50"] > 0 else 0,
    }

def run_throughput_benchmark(name, path, method="GET", payload=None, total_requests=300, concurrency=20):
    print(f"\n--- Running Throughput Benchmark: {name} (Reqs={total_requests}, Concurrency={concurrency}) ---")

    def run_concurrent(url):
        t0 = time.perf_counter()
        completed = 0
        latencies = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = [executor.submit(bench_request, url, path, method, payload) for _ in range(total_requests)]
            for f in concurrent.futures.as_completed(futures):
                lat, status, _ = f.result()
                if status == 200:
                    completed += 1
                    latencies.append(lat)
        total_time = time.perf_counter() - t0
        rps = round(completed / total_time, 2)
        return rps, round(statistics.mean(latencies), 2), completed

    rust_rps, rust_avg, rust_ok = run_concurrent(RUST_URL)
    jvm_rps, jvm_avg, jvm_ok = run_concurrent(JVM_URL)

    print(f"Rust: {rust_rps} req/s (avg {rust_avg}ms, {rust_ok}/{total_requests} OK)")
    print(f"JVM:  {jvm_rps} req/s (avg {jvm_avg}ms, {jvm_ok}/{total_requests} OK)")

    return {
        "test": name,
        "concurrency": concurrency,
        "rust_rps": rust_rps,
        "jvm_rps": jvm_rps,
        "rust_avg_ms": rust_avg,
        "jvm_avg_ms": jvm_avg,
        "throughput_multiplier": round(rust_rps / jvm_rps, 2) if jvm_rps > 0 else 0
    }

def main():
    print("==================================================")
    print(" SUWAYOMI-RUST vs JVM SERVER PERFORMANCE BENCHMARK")
    print("==================================================")

    initial_stats = get_docker_stats()
    image_sizes = get_image_sizes()

    results = {
        "docker_stats": initial_stats,
        "image_sizes": image_sizes,
        "latency_tests": [],
        "throughput_tests": []
    }

    # 1. Categories Relay Query
    cat_query = {"query": "query { categories { nodes { id name } totalCount } }"}
    results["latency_tests"].append(
        run_latency_benchmark("GraphQL: Categories Query", "/api/graphql", "POST", cat_query, n=100)
    )

    # 2. Server Settings Query
    settings_query = {"query": "query { serverSettings { debug webUIInterface webUIFlavor } }"}
    results["latency_tests"].append(
        run_latency_benchmark("GraphQL: Server Settings Query", "/api/graphql", "POST", settings_query, n=100)
    )

    # 3. Sources Query
    sources_query = {"query": "query { sources { nodes { id name lang } totalCount } }"}
    results["latency_tests"].append(
        run_latency_benchmark("GraphQL: Sources Query", "/api/graphql", "POST", sources_query, n=100)
    )

    # 4. Static HTML Index
    results["latency_tests"].append(
        run_latency_benchmark("Static Asset: WebUI index.html", "/", "GET", None, n=100)
    )

    # 5. Throughput under load
    results["throughput_tests"].append(
        run_throughput_benchmark("GraphQL Categories Throughput", "/api/graphql", "POST", cat_query, total_requests=300, concurrency=20)
    )

    results["throughput_tests"].append(
        run_throughput_benchmark("Static Asset Throughput", "/", "GET", None, total_requests=500, concurrency=25)
    )

    final_stats = get_docker_stats()
    results["docker_stats_after"] = final_stats

    with open("benchmark_results.json", "w") as f:
        json.dump(results, f, indent=2)

    print("\nBenchmark completed. Results written to benchmark_results.json")

if __name__ == "__main__":
    main()
