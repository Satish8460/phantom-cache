# scripts/verify_real_cache_ops.ps1
# End-to-end verification of real cache operations, coalescing, and SSE telemetry

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Net.Http

Write-Host "==============================================" -ForegroundColor Cyan
Write-Host " PHANTOMCACHE REAL CACHE OPS & TELEMETRY TEST " -ForegroundColor Cyan
Write-Host "==============================================" -ForegroundColor Cyan

# 1. Start a lightweight HTTP mock origin on port 9099 in native C# thread
$originScript = @"
using System;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;

public class MockOriginServer {
    private static HttpListener listener;
    private static int requestCount = 0;
    private static bool running = true;

    public static void Start() {
        Thread t = new Thread(new ThreadStart(RunLoop));
        t.IsBackground = true;
        t.Start();
    }

    private static void RunLoop() {
        listener = new HttpListener();
        listener.Prefixes.Add("http://127.0.0.1:9099/");
        listener.Start();
        while (running) {
            try {
                HttpListenerContext ctx = listener.GetContext();
                HttpListenerRequest req = ctx.Request;
                HttpListenerResponse res = ctx.Response;
                Interlocked.Increment(ref requestCount);
                string path = req.Url.AbsolutePath;
                
                if (path == "/data/slow") {
                    Thread.Sleep(300); // Artificial delay to exercise single-flight coalescing
                }
                
                res.StatusCode = 200;
                res.Headers["Cache-Control"] = "public, max-age=60";
                res.Headers["X-Origin-Fetches"] = requestCount.ToString();
                byte[] buf = Encoding.UTF8.GetBytes("origin-data-for-" + path);
                res.ContentLength64 = buf.Length;
                res.OutputStream.Write(buf, 0, buf.Length);
                res.OutputStream.Close();
            } catch {
                break;
            }
        }
    }

    public static void Stop() {
        running = false;
        if (listener != null) {
            try { listener.Stop(); } catch {}
        }
    }
}
"@

Add-Type -TypeDefinition $originScript -Language CSharp
[MockOriginServer]::Start()
Start-Sleep -Milliseconds 600
Write-Host "[OK] Mock Origin started on http://127.0.0.1:9099" -ForegroundColor Green

# 2. Launch Gateway binary with ORIGIN_URL
$env:ORIGIN_URL = "http://127.0.0.1:9099"
$env:PHANTOM_BIND_ADDR = "127.0.0.1:8085"
$gatewayProc = Start-Process -FilePath "target\release\phantom-cache-demo.exe" -PassThru -NoNewWindow
Start-Sleep -Seconds 1

try {
    # 3. Verify Health
    $health = (curl.exe -s http://127.0.0.1:8085/health | Out-String).Trim()
    if ($health -ne "OK") { throw "Health check failed: $health" }
    Write-Host "[OK] Gateway Health check passed: $health" -ForegroundColor Green

    # 4. Request 1: Cold GET -> MISS from Cache, fetched from Origin
    $r1Raw = (curl.exe -s -i http://127.0.0.1:8085/api/v1/cache/data/item1 | Out-String)
    if ($r1Raw -notmatch "(?i)x-cache-status:\s*MISS") { throw "Expected x-cache-status: MISS on first request`n$r1Raw" }
    Write-Host "[OK] Request 1: Cold read returned X-Cache-Status: MISS" -ForegroundColor Green

    # 5. Request 2: Warm GET -> HIT from Cache
    $r2Raw = (curl.exe -s -i http://127.0.0.1:8085/api/v1/cache/data/item1 | Out-String)
    if ($r2Raw -notmatch "(?i)x-cache-status:\s*HIT") { throw "Expected x-cache-status: HIT on second request`n$r2Raw" }
    Write-Host "[OK] Request 2: Cached read returned X-Cache-Status: HIT" -ForegroundColor Green

    # 6. Request 3: DELETE key -> Invalidation
    $delRaw = (curl.exe -s -i -X DELETE http://127.0.0.1:8085/api/v1/cache/data/item1 | Out-String)
    if ($delRaw -notmatch "(?i)204 No Content") { throw "Expected 204 No Content on DELETE`n$delRaw" }
    Write-Host "[OK] Request 3: DELETE returned 204 No Content" -ForegroundColor Green

    # 7. Request 4: GET purged key -> MISS again
    $r4Raw = (curl.exe -s -i http://127.0.0.1:8085/api/v1/cache/data/item1 | Out-String)
    if ($r4Raw -notmatch "(?i)x-cache-status:\s*MISS") { throw "Expected x-cache-status: MISS after invalidation`n$r4Raw" }
    Write-Host "[OK] Request 4: Purged read returned X-Cache-Status: MISS" -ForegroundColor Green

    # 8. Single-Flight Coalescing: Concurrent Requests for Slow Key
    Write-Host "Firing 10 concurrent requests to /api/v1/cache/data/slow..."
    $http = New-Object System.Net.Http.HttpClient
    $tasks = 1..10 | ForEach-Object {
        $http.GetAsync("http://127.0.0.1:8085/api/v1/cache/data/slow")
    }
    [System.Threading.Tasks.Task]::WaitAll($tasks)
    
    $statuses = $tasks | ForEach-Object {
        $resp = $_.Result
        $val = ($resp.Headers.GetValues("X-Cache-Status") | Select-Object -First 1)
        $val
    }
    
    $coalescedCount = ($statuses | Where-Object { $_ -eq "COALESCED" }).Count
    $missCount = ($statuses | Where-Object { $_ -eq "MISS" }).Count
    $hitCount = ($statuses | Where-Object { $_ -eq "HIT" }).Count
    Write-Host "[OK] Coalescing Result: $missCount MISS, $coalescedCount COALESCED, $hitCount HIT" -ForegroundColor Green
    if ($coalescedCount -lt 1) { throw "Expected at least 1 coalesced request during stampede, got $coalescedCount" }

    # 9. Verify Atomic Metrics via /api/v1/metrics (Schema 2 with explicit provenance)
    $payload = curl.exe -s http://127.0.0.1:8085/api/v1/metrics | ConvertFrom-Json
    $m = $payload.metrics
    Write-Host "[OK] Live Schema-2 Metrics Verified:" -ForegroundColor Green
    Write-Host "     requests_total: $($m.requests_total.v) (src: $($m.requests_total.src))"
    Write-Host "     cache_hits:     $($m.cache_hits.v) (src: $($m.cache_hits.src))"
    Write-Host "     cache_misses:   $($m.cache_misses.v) (src: $($m.cache_misses.src))"
    Write-Host "     coalesced:      $($m.coalesced_waiters.v) (src: $($m.coalesced_waiters.src))"
    
    if ($m.cache_hits.v -lt 1) { throw "Metrics cache_hits.v expected >= 1, got $($m.cache_hits.v)" }
    if ($m.cache_misses.v -lt 2) { throw "Metrics cache_misses.v expected >= 2, got $($m.cache_misses.v)" }
    if ($m.coalesced_waiters.v -lt 1) { throw "Metrics coalesced_waiters.v expected >= 1, got $($m.coalesced_waiters.v)" }
    if ($m.cache_hits.src -ne "measured") { throw "Metrics src expected 'measured'" }

    # 10. Verify SSE Stream delivers schema 2 payload
    $sseRaw = (curl.exe -s --max-time 2 http://127.0.0.1:8085/api/events?schema=2 | Out-String)
    if ($sseRaw -notmatch "schema.:2") { throw "SSE stream did not emit schema 2 payload" }
    Write-Host "[OK] SSE stream successfully delivered schema 2 event payload" -ForegroundColor Green

    Write-Host "==============================================" -ForegroundColor Green
    Write-Host " ALL REAL CACHE OPERATIONS VERIFIED PASS!     " -ForegroundColor Green
    Write-Host "==============================================" -ForegroundColor Green
}
finally {
    taskkill /f /t /pid $gatewayProc.Id | Out-Null
    [MockOriginServer]::Stop()
}
