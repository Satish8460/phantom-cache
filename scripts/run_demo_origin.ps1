# scripts/run_demo_origin.ps1
param([int]$Port = 9099)
$host.UI.RawUI.WindowTitle = "PhantomCache Mock Origin Server (Port $Port)"
Write-Host "==============================================" -ForegroundColor Green
Write-Host "   PHANTOMCACHE MOCK ORIGIN SERVICE (PORT $Port) " -ForegroundColor Green
Write-Host "==============================================" -ForegroundColor Green
Add-Type -TypeDefinition @"
using System;
using System.Net;
using System.Text;
using System.Threading;

public class DemoOriginServer {
    private static HttpListener listener;
    public static int requestCount = 0;
    public static bool running = true;

    public static void Start(int port) {
        listener = new HttpListener();
        listener.Prefixes.Add("http://127.0.0.1:" + port + "/");
        listener.Start();
        Console.WriteLine("[ORIGIN READY] Listening on http://127.0.0.1:" + port + "/");
        while (running) {
            try {
                HttpListenerContext ctx = listener.GetContext();
                HttpListenerRequest req = ctx.Request;
                HttpListenerResponse res = ctx.Response;
                Interlocked.Increment(ref requestCount);
                string path = req.Url.AbsolutePath;
                Console.WriteLine(string.Format("[{0:HH:mm:ss}] ORIGIN FETCH #{1}: {2} {3}", DateTime.Now, requestCount, req.HttpMethod, path));
                
                if (path == "/data/slow") {
                    Thread.Sleep(300);
                }
                
                res.StatusCode = 200;
                res.Headers["Cache-Control"] = "public, max-age=60";
                res.Headers["X-Origin-Fetches"] = requestCount.ToString();
                byte[] buf = Encoding.UTF8.GetBytes("origin-data-for-" + path);
                res.ContentLength64 = buf.Length;
                res.OutputStream.Write(buf, 0, buf.Length);
                res.OutputStream.Close();
            } catch (Exception ex) {
                Console.WriteLine("[ORIGIN ERROR] " + ex.Message);
                break;
            }
        }
    }
}
"@
[DemoOriginServer]::Start($Port)

