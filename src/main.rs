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
    let webserver = run_webserver();
    let bot = bot::run_bot();
    tokio::select! {
        _ = webserver => {
            println!("Web server stopped")
        }
        _ = bot => {
            println!("Bot stopped")
        }
    }
}

async fn run_webserver() {
    println!("Starting webserver...");
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes)
        .run(([0, 0, 0, 0, 0, 0, 0, 0], 8080))
        .await;
}
