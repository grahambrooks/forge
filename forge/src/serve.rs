//! Forge watch + serve — file watching with optional live-reload HTTP server.
//!
//! `forge watch` — watches .forge and doc files, rebuilds on change
//! `forge serve` — same as watch but also serves the site on localhost with live reload

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, RecursiveMode, Watcher};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::generate::{self, GenerateConfig};
use crate::parser;

/// Run the watch loop — rebuild on .forge / .md file changes.
pub async fn run_watch(
    source: PathBuf,
    out_dir: PathBuf,
    style: String,
    baseline: Option<PathBuf>,
) {
    eprintln!("Watching {} for changes...", source.display());
    eprintln!("Output: {}", out_dir.display());
    eprintln!("Press Ctrl+C to stop.\n");

    // Initial build
    do_build(&source, &out_dir, &style, baseline.as_deref(), None);

    // Watch for changes
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    let source_dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();

    let _watcher = start_watcher(&source_dir, tx);

    let mut last_build = Instant::now();
    while let Some(_event) = rx.recv().await {
        // Debounce: skip if less than 300ms since last build
        if last_build.elapsed() < Duration::from_millis(300) {
            continue;
        }
        last_build = Instant::now();
        eprintln!("\nChange detected, rebuilding...");
        do_build(&source, &out_dir, &style, baseline.as_deref(), None);
    }
}

/// Run the serve loop — watch + HTTP server with live reload.
pub async fn run_serve(
    source: PathBuf,
    out_dir: PathBuf,
    style: String,
    baseline: Option<PathBuf>,
    port: u16,
) {
    let version = Arc::new(AtomicU64::new(1));
    let (notify_tx, _) = watch::channel(1u64);
    let notify_tx = Arc::new(notify_tx);

    eprintln!("Watching {} for changes...", source.display());
    eprintln!("Serving at http://localhost:{}", port);
    eprintln!("Press Ctrl+C to stop.\n");

    // Initial build with live-reload injection
    do_build(&source, &out_dir, &style, baseline.as_deref(), Some(port));

    // Start file watcher
    let (fs_tx, mut fs_rx) = tokio::sync::mpsc::channel(16);
    let source_dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    let _watcher = start_watcher(&source_dir, fs_tx);

    // Spawn rebuild task
    let version_clone = version.clone();
    let notify_clone = notify_tx.clone();
    let source_clone = source.clone();
    let out_clone = out_dir.clone();
    let style_clone = style.clone();
    let baseline_clone = baseline.clone();
    tokio::spawn(async move {
        let mut last_build = Instant::now();
        while let Some(_event) = fs_rx.recv().await {
            if last_build.elapsed() < Duration::from_millis(300) {
                continue;
            }
            last_build = Instant::now();
            eprintln!("\nChange detected, rebuilding...");
            do_build(
                &source_clone,
                &out_clone,
                &style_clone,
                baseline_clone.as_deref(),
                Some(port),
            );
            let v = version_clone.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = notify_clone.send(v);
        }
    });

    // Start HTTP server
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error binding to port {}: {}", port, e);
            std::process::exit(1);
        });

    loop {
        let (mut stream, _addr) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };

        let out_dir = out_dir.clone();
        let version = version.clone();
        let notify_rx = notify_tx.subscribe();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let n = match tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };
            let request = String::from_utf8_lossy(&buf[..n]);

            // Parse the request path
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            if path == "/__reload" {
                // Server-sent events endpoint for live reload
                handle_sse(stream, version, notify_rx).await;
                return;
            }

            // Serve static files
            let file_path = if path == "/" {
                out_dir.join("index.html")
            } else {
                out_dir.join(path.trim_start_matches('/'))
            };

            let (status, content_type, body) = if file_path.is_file() {
                let body = tokio::fs::read(&file_path).await.unwrap_or_default();
                let ct = content_type_for(&file_path);
                ("200 OK", ct, body)
            } else if file_path.join("index.html").is_file() {
                let body = tokio::fs::read(file_path.join("index.html"))
                    .await
                    .unwrap_or_default();
                ("200 OK", "text/html", body)
            } else {
                ("404 Not Found", "text/plain", b"404 Not Found".to_vec())
            };

            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                status,
                content_type,
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(&body).await;
        });
    }
}

// ─── File Watcher ────────────────────────────────────────────────

fn start_watcher(dir: &Path, tx: tokio::sync::mpsc::Sender<()>) -> notify::RecommendedWatcher {
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let dominated = event.paths.iter().any(|p| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                matches!(ext, "forge" | "md" | "forge-rules")
            });
            if dominated {
                let _ = tx.blocking_send(());
            }
        }
    })
    .expect("failed to create file watcher");

    watcher
        .watch(dir, RecursiveMode::Recursive)
        .expect("failed to watch directory");
    watcher
}

// ─── Build ───────────────────────────────────────────────────────

fn do_build(
    source: &Path,
    out_dir: &Path,
    style: &str,
    baseline: Option<&Path>,
    live_reload_port: Option<u16>,
) {
    let text = match std::fs::read_to_string(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("  Error reading {}: {}", source.display(), e);
            return;
        }
    };

    let base_dir = source.parent().unwrap_or(Path::new("."));
    let model = match parser::parse_with_preprocess(&text, base_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  Parse error: {}", e);
            return;
        }
    };

    let diff_result = baseline.and_then(|bp| {
        let bt = std::fs::read_to_string(bp).ok()?;
        let bm = parser::parse_with_preprocess(&bt, bp.parent().unwrap_or(Path::new("."))).ok()?;
        Some(crate::diff::diff(&bm, &model))
    });

    let source_dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    let config = GenerateConfig {
        out_dir: out_dir.to_path_buf(),
        title: None,
        base_url: "/".into(),
        style: style.to_string(),
        source_dir,
    };

    match generate::generate(&model, &config, diff_result.as_ref()) {
        Ok(report) => {
            eprintln!(
                "  Built: {} pages, {} diagrams",
                report.pages, report.diagrams
            );
        }
        Err(e) => {
            eprintln!("  Generate error: {}", e);
            return;
        }
    }

    // Inject live-reload script into index.html if serving
    if let Some(port) = live_reload_port {
        inject_live_reload(out_dir, port);
    }
}

/// Inject a live-reload <script> into all HTML files.
fn inject_live_reload(out_dir: &Path, port: u16) {
    let script = format!(
        r#"<script>
(function(){{
  var ver=null;
  var es=new EventSource('http://localhost:{}/__reload');
  es.onmessage=function(e){{
    var v=e.data.trim();
    if(ver===null){{ ver=v; return; }}
    if(v!==ver){{ location.reload(); }}
  }};
}})();
</script>"#,
        port
    );

    inject_into_html_files(out_dir, &script);
}

fn inject_into_html_files(dir: &Path, script: &str) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            inject_into_html_files(&path, script);
        } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
            if let Ok(html) = std::fs::read_to_string(&path) {
                if !html.contains("__reload") {
                    let patched = html.replace("</body>", &format!("{}\n</body>", script));
                    let _ = std::fs::write(&path, patched);
                }
            }
        }
    }
}

// ─── SSE for live reload ─────────────────────────────────────────

async fn handle_sse(
    mut stream: tokio::net::TcpStream,
    version: Arc<AtomicU64>,
    mut notify_rx: watch::Receiver<u64>,
) {
    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    if stream.write_all(headers.as_bytes()).await.is_err() {
        return;
    }

    // Send current version
    let v = version.load(Ordering::Relaxed);
    let msg = format!("data: {}\n\n", v);
    if stream.write_all(msg.as_bytes()).await.is_err() {
        return;
    }

    // Wait for changes and send updates
    loop {
        if notify_rx.changed().await.is_err() {
            break;
        }
        let v = *notify_rx.borrow();
        let msg = format!("data: {}\n\n", v);
        if stream.write_all(msg.as_bytes()).await.is_err() {
            break;
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────

fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}
