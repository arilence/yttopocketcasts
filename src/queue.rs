use teloxide::{requests::Requester, Bot};

use crate::{
    database::Database,
    downloader,
    types::{BotError, BotErrorKind, BotResult},
    uploader,
    user::User,
};

// From: https://docs.rs/once_cell/latest/once_cell/
// As advised by rust-lang/regex: "Avoid compiling the same regex in a loop"
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

pub struct Queue {
    bot: Bot,
    database: Database,
}

impl Queue {
    pub async fn new(bot: Bot, database: Database) -> Self {
        Queue { bot, database }
    }

    pub async fn start(&self, workers: u64) {
        for _ in 0..workers {
            let bot = self.bot.clone();
            let mut database = self.database.clone();

            tokio::spawn(async move {
                loop {
                    let (_key, value) = match database.wait_for_request().await {
                        Ok((key, value)) => (key, value),
                        // TODO: Indicate there was an error with processing
                        Err(_) => continue,
                    };
                    let data = Queue::get_request(&mut database, value).await;
                    let token = match User::get_token(&mut database, data[0].to_string()).await {
                        Ok(value) => value,
                        // TODO: Indicate there was an error with processing
                        Err(_) => continue,
                    };
                    match Queue::processing_request(&bot, &token, &data).await {
                        // TODO: Indicate there was an error with processing
                        Ok(_) => continue,
                        Err(_) => continue,
                    }
                }
            });
        }
    }

    pub async fn processing_request(
        bot: &Bot,
        token: &String,
        data: &Vec<String>,
    ) -> BotResult<()> {
        bot.send_message(data[1].clone(), "Downloading...").await?;
        let file_info = downloader::download_audio(&data[2]).await?;
        bot.send_message(data[1].clone(), String::from("Uploading..."))
            .await?;
        uploader::upload_audio(&token, &file_info.0, &file_info.1).await?;
        bot.send_message(data[1].clone(), String::from("Done!"))
            .await?;
        Ok(())
    }

    pub async fn get_request(database: &mut Database, key: String) -> Vec<String> {
        match database.get_request(key).await {
            Ok(vec) => vec,
            Err(_) => Vec::new(),
        }
    }

    pub async fn add_request(
        database: &mut Database,
        user_id: String,
        chat_id: String,
        msg_text: String,
    ) -> BotResult<()> {
        // Verify user has a token before adding to queue
        User::get_token(database, user_id.to_string()).await?;
        // Dirty attempt at catching non-youtube links before sending them off to process
        let yt_regex = regex!(
            r#"(?:https?://)?(?:youtu\.be/|(?:www\.|m\.)?youtube\.com/(?:watch|v|embed)(?:\.php)?(?:\?.*v=|/))([a-zA-Z0-9_-]+)"#
        );
        if !yt_regex.is_match(&msg_text) {
            return Err(BotError::new(BotErrorKind::InvalidUrlError));
        }
        database.add_request(user_id, chat_id, msg_text).await?;
        Ok(())
    }
}
