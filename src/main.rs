use warp::Filter;

mod bot;
mod database;
mod downloader;
mod filters;
mod queue;
mod types;
mod uploader;
mod user;

#[tokio::main]
async fn main() {
    // Load environment varilable from .env if available
    dotenvy::dotenv().ok();

    // Fly.io requires a webserver to determine availability
    tokio::spawn(run_webserver());

    bot::run().await;
}

async fn run_webserver() {
    println!("Starting webserver...");
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes)
        .run(([0, 0, 0, 0, 0, 0, 0, 0], 8080))
        .await;
}
