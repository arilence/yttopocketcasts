use std::sync::Arc;

use serde::Deserialize;
use teloxide::{
    dispatching::dialogue::InMemStorage, dptree::case, prelude::*, utils::command::BotCommands,
};
use tokio::sync::RwLock;

use crate::{database::Database, filters, handlers, queue::Queue};

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
pub enum Commands {
    #[command(description = "show intro message")]
    Start,
    #[command(description = "get user id")]
    Id,
    #[command(description = "set auth token")]
    Auth,
    #[command(description = "unset auth token")]
    Clear,
    #[command(description = "cancel auth dialogue")]
    Cancel,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot Commands")]
pub enum AdminCommands {
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

#[derive(Clone, Default)]
pub enum CommandState {
    #[default]
    Start,
    ReceiveAuthToken,
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

    // Update telegram's command list
    match bot.set_my_commands(Commands::bot_commands()).await {
        Ok(_) => (),
        Err(_) => println!("Error: Could not set bot commands on boot"),
    };

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<CommandState>, CommandState>()
        .branch(
            case![CommandState::Start]
                .branch(
                    dptree::entry()
                        .filter_command::<Commands>()
                        //
                        // These commands are available to anyone
                        .branch(case![Commands::Start].endpoint(handlers::start))
                        .branch(case![Commands::Id].endpoint(handlers::id))
                        //
                        // These commands are only available to authorized users
                        .filter_async(filters::is_authorized)
                        .branch(case![Commands::Auth].endpoint(handlers::auth_initiate))
                        .branch(case![Commands::Clear].endpoint(handlers::auth_clear)),
                )
                .branch(
                    dptree::entry()
                        .filter_command::<AdminCommands>()
                        //
                        // These commands are only available to admin users
                        .filter_async(filters::is_admin)
                        .branch(
                            case![AdminCommands::SetCommands].endpoint(handlers::admin_set_command),
                        )
                        .branch(
                            case![AdminCommands::DeleteCache]
                                .endpoint(handlers::admin_delete_cache),
                        ),
                ),
        )
        .branch(
            dptree::entry()
                .filter_command::<Commands>()
                //
                // Cancel command needs to be available regardless of state to stop any ongoing dialogue
                .filter_async(filters::is_authorized)
                .branch(case![Commands::Cancel].endpoint(handlers::auth_cancel)),
        )
        .branch(
            dptree::entry()
                .filter_async(filters::is_authorized)
                .branch(
                    case![CommandState::Start]
                        .filter_async(filters::is_link)
                        .endpoint(handlers::receive_url),
                )
                //
                // Only look for tokens when in "ReceiveAuthToken" state
                .branch(case![CommandState::ReceiveAuthToken].endpoint(handlers::receive_token)),
        )
        .endpoint(handlers::unrecognized);

    Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![
            parameters,
            bot_data,
            InMemStorage::<CommandState>::new()
        ])
        // All message branches failed
        .default_handler(|upd| async move {
            println!("Unhandled update: {:?}", upd);
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
