use std::vec;

use dotenvy::dotenv;
use serde::Deserialize;
use teloxide::{prelude::*, utils::command::BotCommands};
use warp::Filter;

#[tokio::main]
async fn main() {
    // Load environment varilable from .env if available
    dotenv().ok();

    // Fly.io requires a webserver to determine availability
    tokio::join!(run_webserver(), run_bot());
}

async fn run_webserver() {
    println!("Starting webserver...");
    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");

    warp::serve(routes)
        // ipv6 + ipv6 any addr
        .run(([0, 0, 0, 0, 0, 0, 0, 0], 8080))
        .await;
}

async fn run_bot() {
    println!("Starting bot...");
    let bot = Bot::from_env();
    let parameters =
        envy::from_env::<ConfigParameters>().expect("Failed to parse config parameters");
    let handler = Update::filter_message().branch(
        dptree::entry()
            .filter_command::<Commands>()
            .endpoint(commands_handler),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![parameters])
        // All message branches failed
        .default_handler(|_upd| async move {
            // println!("Unhandled update: {:?}", upd);
            println!("Unhandled update");
        })
        // The dispatcher failed
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(Clone, Deserialize)]
struct ConfigParameters {
    // List of users allowed to use the bot
    // TODO: Store these values in a database?
    command_allow_list: Vec<UserId>,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum Commands {
    #[command(description = "shows this message")]
    Help,
    #[command(description = "shows intro message")]
    Start,
    #[command(description = "sets Pocket Casts auth token to upload files")]
    Auth(String),
}

async fn commands_handler(
    cfg: ConfigParameters,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: Commands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        Commands::Help => Commands::descriptions().to_string(),
        Commands::Start => {
            String::from("To start, use `/auth [token]`, with your Pocket Casts auth token.")
        }
        Commands::Auth(token) => {
            let mut response = String::new();
            // Only users in the allow list are able to authenticate themselves
            let incoming_user_id = msg.from().unwrap().id;
            if !cfg
                .command_allow_list
                .iter()
                .any(|&i| i == incoming_user_id)
            {
                response.push_str("You are not authorized.");
            }
            // TODO: Use dialogues instead of command arguments. User issues `/auth` and bot waits for a second message with the auth token.
            else if token.is_empty() {
                response
                    .push_str("Token not found, please include token as part of command argument.")
            } else {
                response.push_str("Token received.")
            }
            response
        }
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}
