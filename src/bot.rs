use std::vec;

use serde::Deserialize;
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::downloader;

// From: https://docs.rs/once_cell/latest/once_cell/
// As advised by rust-lang/regex: "Avoid compiling the same regex in a loop"
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[derive(Clone, Deserialize)]
struct ConfigParameters {
    // List of users allowed to use the bot
    // TODO: Store these values in a database?
    command_allow_list: Vec<UserId>,
}

// TODO: Setup bot_commands() and set_my_commands() to populate the bot's list of known commands
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

pub async fn run_bot() {
    println!("Starting bot...");
    let bot = Bot::from_env();
    let parameters =
        envy::from_env::<ConfigParameters>().expect("Failed to parse config parameters");
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Commands>()
                .endpoint(commands_handler),
        )
        .branch(Update::filter_message().endpoint(text_handler));

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
            String::from("This bot downloads Youtube videos as audio files and uploads them to your personal Pocket Casts account.\n\nTo start: /auth [pocketcasts token]")
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
                response.push_str("You are not authorized to use this command.");
            }
            // TODO: Use dialogues instead of command arguments. User issues `/auth` and bot waits for a second message with the auth token.
            else if token.is_empty() {
                response.push_str("Invalid command, token not found.\n\nUsage: /auth [token]");
            } else {
                let jwt_regex = regex!(r#"^([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_\-\+/=]*)"#);
                if jwt_regex.is_match(&token) {
                    response.push_str("Token received. (But not really just yet)")
                } else {
                    response.push_str("Token is not a valid JWT.\n\nUsage: /auth [token]")
                }
            }
            response
        }
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn text_handler(
    cfg: ConfigParameters,
    bot: Bot,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    // Only users in the allow list are able to authenticate themselves
    let incoming_user_id = msg.from().unwrap().id;
    if !cfg
        .command_allow_list
        .iter()
        .any(|&i| i == incoming_user_id)
    {
        bot.send_message(
            msg.chat.id,
            String::from("You are not authorized to use this command."),
        )
        .await?;
        return Ok(());
    }
    // Dirty attempt at catching non-youtube links before sending them off to process
    let yt_regex = regex!(
        r#"(?:https?://)?(?:youtu\.be/|(?:www\.|m\.)?youtube\.com/(?:watch|v|embed)(?:\.php)?(?:\?.*v=|/))([a-zA-Z0-9_-]+)"#
    );
    let incoming_text = match msg.text() {
        Some(val) => val,
        None => return Ok(()),
    };
    if !yt_regex.is_match(incoming_text) {
        bot.send_message(
            msg.chat.id,
            String::from("Please send a valid youtube link."),
        )
        .await?;
        return Ok(());
    }
    // TODO: Check if user has set their auth token before sending a link
    bot.send_message(
        msg.chat.id,
        String::from("Valid youtube link. Starting processing..."),
    )
    .await?;
    let incoming_string = String::from(incoming_text);
    tokio::spawn(async move {
        // TODO: Implement a processing queue
        downloader::download_audio(&incoming_string).await;
        bot.send_message(msg.chat.id, String::from("Finished processing!"))
            .await
            .unwrap();
    });
    Ok(())
}
