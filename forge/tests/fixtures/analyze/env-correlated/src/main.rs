use axum::{Router, routing::get};

fn main() {
    let _db = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let _cache = std::env::var("REDIS_URL").unwrap_or_default();
    let _r: Router<()> = Router::new().route("/health", get(|| async { "ok" }));
}
