use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("missing OAuth client credential: {0}")]
    MissingCredential(&'static str),

    #[error("unknown calendar source: {0}")]
    UnknownSource(String),

    #[error("OAuth flow failed: {0}")]
    OAuth(String),

    #[error("OAuth state mismatch")]
    StateMismatch,

    #[error("OAuth callback timeout")]
    CallbackTimeout,

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("keyring error: {0}")]
    Keyring(String),

    #[error("CalDAV error: {0}")]
    CalDav(String),

    #[error("token expired and no refresh token available")]
    TokenExpired,

    #[error("not authenticated for source: {0}")]
    NotAuthenticated(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::Keyring(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
