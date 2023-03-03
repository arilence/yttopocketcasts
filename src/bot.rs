use std::{path::Path, sync::Arc};

use serde::Deserialize;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::{fs, sync::RwLock};

use crate::{database::Database, filters, queue::Queue, user::User};

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
    #[command(description = "show intro message")]
    Start,
    #[command(description = "get user id")]
    Id,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum TrustedCommands {
    #[command(description = "set Pocket Casts auth token")]
    Auth(String),
    #[command(description = "unset auth token")]
    Clear,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
enum AdminCommands {
    #[command(description = "update list of bot commands on Telegram")]
    SetCommands,
    // NOTE: This deletes all files without waiting for other processes to finish
    #[command(description = "delete all cached files")]
    DeleteCache,
}

pub struct BotData {
    pub db_client: Database,
}

impl BotData {
    pub async fn new(db_client: Database) -> Self {
        Self { db_client }
    }
}

pub async fn run() {
    println!("Starting bot...");

    let parameters =
        envy::from_env::<ConfigParameters>().expect("Failed to parse config parameters");

    let db_client = Database::new().await;

    let bot = teloxide::Bot::from_env();
    let bot_data: Arc<RwLock<BotData>> =
        Arc::new(RwLock::new(BotData::new(db_client.clone()).await));

    let queue = Queue::new(bot.clone(), db_client.clone()).await;
    let workers = 2;
    queue.start(workers).await;

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

    Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![parameters, bot_data])
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
    bot: teloxide::Bot,
    _bot_data: Arc<RwLock<BotData>>,
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
    bot: teloxide::Bot,
    bot_data: Arc<RwLock<BotData>>,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: TrustedCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        TrustedCommands::Auth(token) => command_auth(&msg, &bot_data, token).await,
        TrustedCommands::Clear => command_clear(&msg, &bot_data).await,
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_admin_commands(
    _cfg: ConfigParameters,
    bot: teloxide::Bot,
    _bot_data: Arc<RwLock<BotData>>,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: AdminCommands,
) -> Result<(), teloxide::RequestError> {
    let text = match cmd {
        AdminCommands::SetCommands => {
            let commands = [
                GeneralCommands::bot_commands().as_slice(),
                TrustedCommands::bot_commands().as_slice(),
            ]
            .concat();
            bot.set_my_commands(commands).await?;
            String::from("Commands updated")
        }
        AdminCommands::DeleteCache => {
            let path = Path::new("/tmp/.cache/");
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
    bot: teloxide::Bot,
    bot_data: Arc<RwLock<BotData>>,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = command_link(cfg, bot.clone(), &bot_data, msg.clone()).await;
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_unrecognized_messages(
    _cfg: ConfigParameters,
    bot: teloxide::Bot,
    _bot_data: Arc<RwLock<BotData>>,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = String::from("Command not found. Use /start");
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_unauthorized_message(
    _cfg: ConfigParameters,
    bot: teloxide::Bot,
    _bot_data: Arc<RwLock<BotData>>,
    _me: teloxide::types::Me,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let text = String::from("You are not authorized to use this command. Use /start");
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn command_auth(msg: &Message, bot_data: &Arc<RwLock<BotData>>, token: String) -> String {
    // TODO: Use dialogues instead of command arguments. User issues `/auth` and bot waits for a second message with the auth token.
    let mut db_client = bot_data.read().await.db_client.clone();
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => return String::from("Something went wrong. Please try again."),
    };

    match User::set_token(&mut db_client, user_id.to_string(), token).await {
        Ok(_) => String::from("Token set. Start sending me some youtube videos."),
        Err(error) => match error.kind {
            crate::types::BotErrorKind::EmptyTokenError => {
                String::from("Please provide a token. /auth [token]")
            }
            crate::types::BotErrorKind::InvalidTokenError => {
                String::from("Token doesn't seem to be a valid JWT. /auth [token]")
            }
            crate::types::BotErrorKind::RedisError => {
                String::from("Unable to save auth token. Please try again.")
            }
            _ => String::from("Something went wrong. Please try again."),
        },
    }
}

async fn command_clear(msg: &Message, bot_data: &Arc<RwLock<BotData>>) -> String {
    let mut db_client = bot_data.read().await.db_client.clone();
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => return String::from("Something went wrong. Please try again."),
    };

    match User::delete_token(&mut db_client, user_id.to_string()).await {
        Ok(_) => String::from("Token removed successfully."),
        Err(error) => match error.kind {
            crate::types::BotErrorKind::RedisError => {
                String::from("Unable to remove token. Please try again.")
            }
            _ => String::from("Something went wrong. Please try again."),
        },
    }
}

async fn command_link(
    _cfg: ConfigParameters,
    _bot: teloxide::Bot,
    bot_data: &Arc<RwLock<BotData>>,
    msg: Message,
) -> String {
    let mut db_client = bot_data.read().await.db_client.clone();
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => return String::from("Something went wrong. Please try again."),
    };
    let chat_id = msg.chat.id;
    let msg_text = msg.text().unwrap_or_default();

    match Queue::add_request(
        &mut db_client,
        user_id.to_string(),
        chat_id.to_string(),
        msg_text.to_string(),
    )
    .await
    {
        Ok(_) => return String::from("Waiting to be processed..."),
        Err(error) => match error.kind {
            crate::types::BotErrorKind::EmptyTokenError => {
                String::from("Please provide a token. /auth [token]")
            }
            crate::types::BotErrorKind::InvalidTokenError => {
                String::from("Token doesn't seem to be a valid JWT. /auth [token]")
            }
            crate::types::BotErrorKind::InvalidUrlError => {
                return String::from("Please send a valid youtube link.");
            }
            _ => String::from("Unable to process request"),
        },
    }
}
