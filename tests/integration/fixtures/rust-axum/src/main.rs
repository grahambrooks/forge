use axum::{routing::get, Router};

async fn list_payments() -> &'static str {
    "[]"
}

async fn create_payment() -> &'static str {
    "{}"
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/payments", get(list_payments))
        .route("/payments", post(create_payment))
        .route("/health", get(health));

    let db_url = "postgres://localhost:5432/payments";
    println!("Connecting to {}", db_url);
}
