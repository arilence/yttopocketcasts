use crate::{
    database::Database,
    types::{BotError, BotErrorKind, BotResult},
};

// From: https://docs.rs/once_cell/latest/once_cell/
// As advised by rust-lang/regex: "Avoid compiling the same regex in a loop"
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

pub struct User {}

impl User {
    // TODO: Check if token already exists and return an "Update success" message
    pub async fn set_token(
        database: &mut Database,
        user_id: String,
        token: String,
    ) -> BotResult<()> {
        if token.is_empty() {
            return Err(BotError::new(BotErrorKind::EmptyTokenError));
        }
        let jwt_regex = regex!(r#"^([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_=]+)\.([a-zA-Z0-9_\-\+/=]*)"#);
        if !jwt_regex.is_match(&token) {
            return Err(BotError::new(BotErrorKind::InvalidTokenError));
        }
        database.set_token(user_id, token).await?;
        Ok(())
    }
    pub async fn get_token(database: &mut Database, user_id: String) -> BotResult<String> {
        Ok(database.get_token(user_id).await?)
    }
    pub async fn delete_token(database: &mut Database, user_id: String) -> BotResult<()> {
        Ok(database.delete_token(user_id).await?)
    }
}
