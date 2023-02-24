use dotenvy::dotenv;
use warp::Filter;

mod bot;
mod downloader;
mod filters;
mod uploader;

// TODO: Implement actual error types
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() {
    // Load environment varilable from .env if available
    dotenv().ok();

    // Fly.io requires a webserver to determine availability
    tokio::spawn(run_webserver());

    bot::run_bot().await;
}

async fn run_webserver() {
    println!("Starting webserver...");
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes)
        .run(([0, 0, 0, 0, 0, 0, 0, 0], 8080))
        .await;
}
