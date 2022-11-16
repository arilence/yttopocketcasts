use dotenvy::dotenv;
use warp::Filter;

mod bot;

#[tokio::main]
async fn main() {
    // Load environment varilable from .env if available
    dotenv().ok();

    // Fly.io requires a webserver to determine availability
    tokio::join!(run_webserver(), bot::run_bot());
}

async fn run_webserver() {
    println!("Starting webserver...");
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes)
        .run(([0, 0, 0, 0, 0, 0, 0, 0], 8080))
        .await;
}
