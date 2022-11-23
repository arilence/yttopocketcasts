use std::{collections::HashMap, path::Path, sync::Arc, vec};

use serde::Deserialize;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::{fs, sync::RwLock};

use crate::{downloader, filters, uploader};

// From: https://docs.rs/once_cell/latest/once_cell/
// As advised by rust-lang/regex: "Avoid compiling the same regex in a loop"
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

// Prevents serde from panicking when trying to parse env vars that don't exist
fn default_user_ids() -> Vec<UserId> {
    Vec::new()
}

#[derive(Clone, Deserialize)]
pub struct ConfigParameters {
    // TODO: Store these values in a database?
    // List of users allowed to use the bot
    #[serde(default = "default_user_ids")]
    pub trusted_user_ids: Vec<UserId>,
    // List of users who are allowed to use Admin commands
    #[serde(default = "default_user_ids")]
    pub admin_user_ids: Vec<UserId>,
}

// TODO: Setup bot_commands() and set_my_commands() to populate the bot's list of known commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum GeneralCommands {
    #[command(description = "shows intro message")]
    Start,
    #[command(description = "returns user id")]
    Id,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum TrustedCommands {
    #[command(description = "sets Pocket Casts auth token to upload files")]
    Auth(String),
    #[command(description = "removes associated auth token")]
    Clear,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum AdminCommands {
    // NOTE: This deletes all files without waiting for other processes to finish
    #[command(description = "deletes all cached files")]
    DeleteCache,
}

pub async fn run_bot() {
    println!("Starting bot...");
    let bot = Bot::from_env();
    let parameters =
        envy::from_env::<ConfigParameters>().expect("Failed to parse config parameters");
    let user_tokens: Arc<RwLock<HashMap<UserId, String>>> = Arc::new(RwLock::new(HashMap::new()));

    let handler = Update::filter_message()
        // General commands: Anyone can use these commands
        .branch(
            dptree::entry()
                .filter_command::<GeneralCommands>()
                .endpoint(handle_general_commands),
        )
        // Trusted commands: both trusted and admin users can use these
        .branch(
            dptree::entry()
                .filter_command::<TrustedCommands>()
                .branch(dptree::filter_async(filters::is_trusted).endpoint(handle_trusted_commands))
                .endpoint(handle_unauthorized_message),
        )
        // Admin commands: only admin users can use these
        // These commands are hidden to all non-admin users and appear as "Unknown command"
        .branch(
            dptree::entry()
                .filter_command::<AdminCommands>()
                .branch(dptree::filter_async(filters::is_admin).endpoint(handle_admin_commands))
                .endpoint(handle_unrecognized_messages),
        )
        // Match non-command messages such as Youtube links
        // Limited to trusted and admin users
        .branch(
            Update::filter_message().branch(
                dptree::filter_async(filters::is_trusted)
                    .filter_async(filters::is_link)
                    .endpoint(handle_link_messages),
            ),
        )
        // Unrecognized text
        .branch(Update::filter_message().endpoint(handle_unrecognized_messages));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![parameters, Arc::clone(&user_tokens)])
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

async fn handle_general_commands(
    _cfg: ConfigParameters,
    _user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: GeneralCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        GeneralCommands::Start => {
            String::from("This bot sends Youtube videos as audio podcasts to your personal Pocket Casts files section.\n\nTo get user id: /id\n\nTo start: /auth [pocketcasts token]")
        }
        GeneralCommands::Id => {
            let user_id = msg.from().unwrap().id;
            format!("User Id: {}", user_id)
        }
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_trusted_commands(
    _cfg: ConfigParameters,
    user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: TrustedCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        TrustedCommands::Auth(token) => command_auth(&msg, user_tokens, token).await,
        TrustedCommands::Clear => command_clear(&msg, user_tokens).await,
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_admin_commands(
    _cfg: ConfigParameters,
    _user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: AdminCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        AdminCommands::DeleteCache => {
            let path = Path::new(".cache/");
            let mut reader = fs::read_dir(path).await?;
            while let Ok(entry) = reader.next_entry().await {
                match entry {
                    Some(val) => {
                        fs::remove_file(val.path()).await?;
                    }
                    None => break,
                }
            }
            String::from("Cleared .cache folder")
        }
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_link_messages(
    cfg: ConfigParameters,
    user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = command_link(cfg, user_tokens, bot.clone(), msg.clone()).await;
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_unrecognized_messages(
    _cfg: ConfigParameters,
    _user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = String::from("Command not found.\n\nUse /start");
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_unauthorized_message(
    _cfg: ConfigParameters,
    _user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = String::from("You are not authorized to use this command.\n\nUse /start");
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn command_auth(
    msg: &Message,
    user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    token: String,
) -> String {
    // TODO: Use dialogues instead of command arguments. User issues `/auth` and bot waits for a second message with the auth token.
    if token.is_empty() {
        return String::from("Please provide a token.\n\nUsage: /auth [token]");
    }
    let jwt_regex = regex!(r#"^([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_\-\+/=]*)"#);
    if !jwt_regex.is_match(&token) {
        return String::from("Token doesn't seem to be a valid JWT.\n\nUsage: /auth [token]");
    }
    let user_id = msg.from().unwrap().id;
    let mut tokens = user_tokens.write().await;
    tokens.insert(user_id, token);
    String::from("Token set.\n\nStart sending me some youtube videos.")
}

async fn command_clear(msg: &Message, user_tokens: Arc<RwLock<HashMap<UserId, String>>>) -> String {
    let user_id = msg.from().unwrap().id;
    let mut tokens = user_tokens.write().await;
    match tokens.remove(&user_id) {
        Some(_) => String::from("Token removed."),
        None => String::from("No token found associated with your user id."),
    }
}

async fn command_link(
    _cfg: ConfigParameters,
    user_tokens: Arc<RwLock<HashMap<UserId, String>>>,
    bot: Bot,
    msg: Message,
) -> String {
    let incoming_text = msg.text().unwrap_or_default();
    // Dirty attempt at catching non-youtube links before sending them off to process
    let yt_regex = regex!(
        r#"(?:https?://)?(?:youtu\.be/|(?:www\.|m\.)?youtube\.com/(?:watch|v|embed)(?:\.php)?(?:\?.*v=|/))([a-zA-Z0-9_-]+)"#
    );
    if !yt_regex.is_match(incoming_text) {
        return String::from("Please send a valid youtube link.");
    }
    // Check if user has set their auth token before sending a link
    let user_id = msg.from().unwrap().id;
    let tokens = user_tokens.read().await;
    let token = match tokens.get(&user_id) {
        Some(val) => val.clone(),
        None => {
            return String::from("Please set a token before sending videos\n\nUse: /auth [token]");
        }
    };
    let url_string = String::from(incoming_text);
    tokio::spawn(async move {
        // TODO: Implement a processing queue
        let file_info = downloader::download_audio(&url_string)
            .await
            .expect("Failed to download file");
        bot.send_message(msg.chat.id, String::from("Uploading..."))
            .await
            .unwrap();
        uploader::upload_audio(&token, &file_info.0, &file_info.1)
            .await
            .expect("Failed to upload file");
        bot.send_message(msg.chat.id, String::from("Done!"))
            .await
            .unwrap();
    });
    String::from("Downloading...")
}
