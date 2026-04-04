use actix_web::{web, App, HttpServer};

#[get("/api/status")]
async fn status() -> &'static str {
    "ok"
}

#[post("/api/orders")]
async fn create_order() -> &'static str {
    "{}"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = "postgres://db:5432/app";
    println!("db: {}", db);

    HttpServer::new(|| App::new())
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
