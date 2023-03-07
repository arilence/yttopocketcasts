use std::{
    fmt::{Display, Formatter},
    string::FromUtf8Error,
};

use teloxide::{dispatching::dialogue::InMemStorage, prelude::Dialogue};
use tokio::io;

use crate::bot::CommandState;

pub type BotDialogue = Dialogue<CommandState, InMemStorage<CommandState>>;

pub type BotResult<T> = Result<T, BotError>;

#[derive(Clone, Debug)]
pub struct BotError {
    pub kind: BotErrorKind,
}

impl BotError {
    pub fn new(kind: BotErrorKind) -> BotError {
        BotError { kind }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum BotErrorKind {
    DownloadError,
    EmptyTokenError,
    InvalidTokenError,
    InvalidUrlError,
    IoError,
    RedisError,
    TelegramError,
    TypeError,
    UploadError,
    WebClientError,
}

impl std::error::Error for BotError {}

impl From<FromUtf8Error> for BotError {
    fn from(_: FromUtf8Error) -> BotError {
        BotError {
            kind: BotErrorKind::TypeError,
        }
    }
}

impl From<io::Error> for BotError {
    fn from(_: io::Error) -> BotError {
        BotError {
            kind: BotErrorKind::IoError,
        }
    }
}

impl From<redis::RedisError> for BotError {
    fn from(_: redis::RedisError) -> BotError {
        BotError {
            kind: BotErrorKind::RedisError,
        }
    }
}

impl From<reqwest::Error> for BotError {
    fn from(_: reqwest::Error) -> BotError {
        BotError {
            kind: BotErrorKind::WebClientError,
        }
    }
}

impl From<teloxide::RequestError> for BotError {
    fn from(_: teloxide::RequestError) -> BotError {
        BotError {
            kind: BotErrorKind::TelegramError,
        }
    }
}

impl Display for BotError {
    fn fmt(&self, _f: &mut Formatter) -> std::fmt::Result {
        match self.kind {
            BotErrorKind::DownloadError => todo!(),
            BotErrorKind::EmptyTokenError => todo!(),
            BotErrorKind::InvalidTokenError => todo!(),
            BotErrorKind::InvalidUrlError => todo!(),
            BotErrorKind::IoError => todo!(),
            BotErrorKind::RedisError => todo!(),
            BotErrorKind::TelegramError => todo!(),
            BotErrorKind::TypeError => todo!(),
            BotErrorKind::UploadError => todo!(),
            BotErrorKind::WebClientError => todo!(),
        }
    }
}
