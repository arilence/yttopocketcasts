use std::{env, vec};

use serde::Deserialize;
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::{downloader, uploader};

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
    trusted_user_ids: Vec<UserId>,
}

// TODO: Setup bot_commands() and set_my_commands() to populate the bot's list of known commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum GeneralCommands {
    #[command(description = "shows intro message")]
    Start,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum TrustedCommands {
    #[command(description = "sets Pocket Casts auth token to upload files")]
    Auth(String),
}

pub async fn run_bot() {
    println!("Starting bot...");
    let bot = Bot::from_env();
    let parameters =
        envy::from_env::<ConfigParameters>().expect("Failed to parse config parameters");

    let handler = Update::filter_message()
        .branch(
            // Anyone can use the general commands
            dptree::entry()
                .filter_command::<GeneralCommands>()
                .endpoint(general_commands_handler),
        )
        .branch(
            // Only user id's found in the Trusted List can use Trusted Commands
            dptree::filter(|cfg: ConfigParameters, msg: Message| {
                msg.from()
                    .map(|user| cfg.trusted_user_ids.iter().any(|&i| i == user.id))
                    .unwrap_or_default()
            })
            .filter_command::<TrustedCommands>()
            .endpoint(trusted_commands_handler),
        )
        // Any message that isn't matched as a command goes here
        // In most cases the message will be a youtube link
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

async fn general_commands_handler(
    _cfg: ConfigParameters,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: GeneralCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        GeneralCommands::Start => {
            String::from("This bot downloads Youtube videos as audio files and uploads them to your personal Pocket Casts account.\n\nTo start: /auth [pocketcasts token]")
        }
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn trusted_commands_handler(
    _cfg: ConfigParameters,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: TrustedCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        TrustedCommands::Auth(token) => {
            let mut response = String::new();
            // TODO: Use dialogues instead of command arguments. User issues `/auth` and bot waits for a second message with the auth token.
            if token.is_empty() {
                response.push_str("Invalid command, token not found.\n\nUsage: /auth [token]");
            } else {
                let jwt_regex =
                    regex!(r#"^([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_\-\+/=]*)"#);
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
    _cfg: ConfigParameters,
    bot: Bot,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
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
        let file_info = downloader::download_audio(&incoming_string)
            .await
            .expect("yt-dlp failed to download file");
        bot.send_message(
            msg.chat.id,
            String::from("Finished downloading. Now uploading..."),
        )
        .await
        .unwrap();
        let token: String = env::var("POCKETCASTS_TOKEN").expect("Pocket Casts token not set");
        uploader::upload_audio(&token, &file_info.0, &file_info.1)
            .await
            .expect("Failed to upload file");
        bot.send_message(msg.chat.id, String::from("Done!"))
            .await
            .unwrap();
    });
    Ok(())
}
