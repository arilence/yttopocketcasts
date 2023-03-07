use std::sync::Arc;

use teloxide::{requests::Requester, types::Message, utils::command::BotCommands};
use tokio::sync::RwLock;

use crate::{
    bot::{BotData, CommandState, Commands},
    queue::Queue,
    types::BotDialogue,
    user::User,
};

pub async fn unrecognized(bot: teloxide::Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "Command not found.").await?;
    Ok(())
}

pub async fn start(bot: teloxide::Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "This bot sends Youtube videos as audio podcasts to your personal Pocket Casts files section.\n\nTo get user id: /id\n\nTo start: /auth")
        .await?;
    Ok(())
}

pub async fn id(bot: teloxide::Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => {
            bot.send_message(msg.chat.id, "Something went wrong. Please try again.")
                .await?;
            return Ok(());
        }
    };
    let output = format!("User Id: {}", user_id);
    bot.send_message(msg.chat.id, output).await?;
    Ok(())
}

pub async fn auth_clear(
    bot: teloxide::Bot,
    dialogue: BotDialogue,
    msg: Message,
    bot_data: Arc<RwLock<BotData>>,
) -> Result<(), teloxide::RequestError> {
    let mut db_client = bot_data.read().await.db_client.clone();
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => {
            bot.send_message(msg.chat.id, "Something went wrong. Please try again.")
                .await?;
            return Ok(());
        }
    };

    let output = match User::delete_token(&mut db_client, user_id.to_string()).await {
        Ok(_) => String::from("Token removed successfully."),
        Err(error) => match error.kind {
            crate::types::BotErrorKind::RedisError => {
                String::from("Unable to remove token. Please try again.")
            }
            _ => String::from("Something went wrong. Please try again."),
        },
    };
    dialogue.exit().await.unwrap();
    bot.send_message(msg.chat.id, output).await?;
    Ok(())
}

pub async fn auth_initiate(
    bot: teloxide::Bot,
    dialogue: BotDialogue,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "What is your auth token? /cancel to stop")
        .await?;
    dialogue
        .update(CommandState::ReceiveAuthToken)
        .await
        .unwrap();
    Ok(())
}

pub async fn receive_token(
    bot: teloxide::Bot,
    dialogue: BotDialogue,
    msg: Message,
    bot_data: Arc<RwLock<BotData>>,
) -> Result<(), teloxide::RequestError> {
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => {
            bot.send_message(msg.chat.id, "Something went wrong. Please send again.")
                .await?;
            return Ok(());
        }
    };

    let token = msg.text().unwrap_or_default().to_string();
    let mut db_client = bot_data.read().await.db_client.clone();

    let msg_response = match User::set_token(&mut db_client, user_id.to_string(), token).await {
        Ok(_) => {
            dialogue.exit().await.unwrap();
            String::from("Token set. Start sending me some youtube videos.")
        }
        Err(error) => match error.kind {
            crate::types::BotErrorKind::EmptyTokenError => String::from("Please send some text"),
            crate::types::BotErrorKind::InvalidTokenError => {
                String::from("Please send a valid auth token")
            }
            crate::types::BotErrorKind::RedisError => {
                String::from("Unable to save auth token. Please send again.")
            }
            _ => String::from("Something went wrong. Please send again."),
        },
    };
    bot.send_message(msg.chat.id, msg_response).await?;
    Ok(())
}

pub async fn auth_cancel(
    bot: teloxide::Bot,
    dialogue: BotDialogue,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let output = match dialogue.exit().await {
        Ok(_) => String::from("Authentication cancelled."),
        Err(_) => String::from("Something went wrong. Please try again."),
    };
    bot.send_message(msg.chat.id, output).await?;
    Ok(())
}

pub async fn receive_url(
    bot: teloxide::Bot,
    msg: Message,
    bot_data: Arc<RwLock<BotData>>,
) -> Result<(), teloxide::RequestError> {
    let mut db_client = bot_data.read().await.db_client.clone();
    let user_id = match msg.from() {
        Some(msg) => msg.id,
        None => {
            bot.send_message(msg.chat.id, "Something went wrong. Please try again.")
                .await?;
            return Ok(());
        }
    };
    let chat_id = msg.chat.id;
    let msg_text = msg.text().unwrap_or_default();

    let output = match Queue::add_request(
        &mut db_client,
        user_id.to_string(),
        chat_id.to_string(),
        msg_text.to_string(),
    )
    .await
    {
        Ok(_) => String::from("Waiting to be processed..."),
        Err(error) => match error.kind {
            crate::types::BotErrorKind::EmptyTokenError => {
                String::from("Please set an /auth token before sending URLs.")
            }
            crate::types::BotErrorKind::InvalidTokenError => {
                String::from("Please set a valid /auth token before sending URLs")
            }
            crate::types::BotErrorKind::InvalidUrlError => {
                String::from("Please send a valid youtube link.")
            }
            _ => String::from("Unable to process request. Please try again."),
        },
    };
    bot.send_message(msg.chat.id, output).await?;
    Ok(())
}

pub async fn admin_set_command(
    bot: teloxide::Bot,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    bot.set_my_commands(Commands::bot_commands()).await?;
    bot.send_message(msg.chat.id, "Bot commands updated.")
        .await?;
    Ok(())
}

pub async fn admin_delete_cache(
    bot: teloxide::Bot,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let path = std::path::Path::new("/tmp/.cache/");
    let mut reader = tokio::fs::read_dir(path).await?;
    while let Ok(entry) = reader.next_entry().await {
        match entry {
            Some(val) => {
                tokio::fs::remove_file(val.path()).await?;
            }
            None => break,
        }
    }
    bot.send_message(msg.chat.id, "Cleared `.cache` folder.")
        .await?;
    Ok(())
}
