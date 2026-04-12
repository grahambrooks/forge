fn main() {
    let _db = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    println!("billing booted");
}
