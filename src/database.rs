use std::env;

use redis::{aio::MultiplexedConnection, AsyncCommands, Client};

use crate::types::{BotError, BotErrorKind, BotResult};

#[derive(Clone)]
pub struct Database {
    pub client: Client,
    pub publish_conn: MultiplexedConnection,
    pub blocking_conn: MultiplexedConnection,
}

impl Database {
    pub async fn new() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        let client = redis::Client::open(redis_url).unwrap();
        let publish_conn = client.get_multiplexed_async_connection().await.unwrap();
        let blocking_conn = client.get_multiplexed_async_connection().await.unwrap();

        Database {
            client,
            publish_conn,
            blocking_conn,
        }
    }

    pub async fn get_token(&mut self, user_id: String) -> BotResult<String> {
        let id_string = format!("user-token:{}", user_id);
        match self.publish_conn.get(id_string).await {
            Ok(value) => Ok(value),
            Err(error) => match error.kind() {
                redis::ErrorKind::TypeError => Err(BotError::new(BotErrorKind::EmptyTokenError)),
                _ => Err(BotError::new(BotErrorKind::RedisError)),
            },
        }
    }

    pub async fn set_token(&mut self, user_id: String, token: String) -> BotResult<()> {
        let id_string = format!("user-token:{}", user_id);
        match self
            .publish_conn
            .set::<String, String, String>(id_string, token)
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(BotError::new(BotErrorKind::RedisError)),
        }
    }

    pub async fn delete_token(&mut self, user_id: String) -> BotResult<()> {
        let id_string = format!("user-token:{}", user_id);
        match self.publish_conn.del::<String, i64>(id_string).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BotError::new(BotErrorKind::RedisError)),
        }
    }

    pub async fn wait_for_request(&mut self) -> BotResult<(String, String)> {
        let timeout = 0;
        match self
            .blocking_conn
            .brpop::<&str, (String, String)>("yt_processing", timeout)
            .await
        {
            Ok(value) => Ok(value),
            Err(_) => Err(BotError::new(BotErrorKind::RedisError)),
        }
    }

    pub async fn get_request(&mut self, key: String) -> BotResult<Vec<String>> {
        match self
            .publish_conn
            .hget::<String, &[&str], Vec<String>>(key, &["user_id", "chat_id", "url"])
            .await
        {
            Ok(data) => Ok(data),
            Err(_) => Err(BotError::new(BotErrorKind::RedisError)),
        }
    }

    pub async fn add_request(
        &mut self,
        user_id: String,
        chat_id: String,
        url: String,
    ) -> BotResult<()> {
        let processing_id: i64 = match self
            .publish_conn
            .incr::<&str, i64, i64>("next_processing_id", 1)
            .await
        {
            Ok(value) => value,
            Err(_) => return Err(BotError::new(BotErrorKind::RedisError)),
        };
        let processing_key = format!("yt_processing:{}", processing_id);

        // Add the request to the database, but the workers won't know about it just yet
        match self
            .publish_conn
            .hset_multiple::<&String, &str, String, ()>(
                &processing_key,
                &[("user_id", user_id), ("chat_id", chat_id), ("url", url)],
            )
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(BotError::new(BotErrorKind::RedisError)),
        };

        // Let the workers know there's a new request to be fulfilled
        match self
            .publish_conn
            .lpush::<&str, &String, ()>("yt_processing", &processing_key)
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => return Err(BotError::new(BotErrorKind::RedisError)),
        }
    }
}
