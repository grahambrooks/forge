//! `forge watch` and `forge serve` — both wrap the live-reload server.

use std::path::PathBuf;

use crate::serve;

pub fn cmd_watch(source: PathBuf, out: PathBuf, style: String, baseline: Option<PathBuf>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(serve::run_watch(source, out, style, baseline));
}

pub fn cmd_serve(
    source: PathBuf,
    out: PathBuf,
    style: String,
    port: u16,
    baseline: Option<PathBuf>,
    present: bool,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    if present {
        eprintln!("Presentation mode: http://localhost:{}/present.html", port);
    }
    rt.block_on(serve::run_serve(
        source, out, style, baseline, port, present,
    ));
}
