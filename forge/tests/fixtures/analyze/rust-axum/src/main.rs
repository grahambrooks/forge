use axum::{Router, routing::get, routing::post};

async fn list_payments() -> &'static str { "[]" }
async fn create_payment() -> &'static str { "ok" }

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/payments", get(list_payments))
        .route("/payments", post(create_payment))
        .route("/health", get(|| async { "ok" }));

    // postgres://user:pw@localhost/payments
    let _db_url = std::env::var("DATABASE_URL").unwrap_or_default();

    axum::serve(tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(), app)
        .await
        .unwrap();
}
