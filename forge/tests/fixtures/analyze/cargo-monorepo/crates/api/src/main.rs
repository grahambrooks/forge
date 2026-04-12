use axum::{Router, routing::get};

fn main() {
    let _r = Router::<()>::new().route("/status", get(|| async { "ok" }));
}
