use serde::ser::SerializeStruct;
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

    /// HTTP request reached the server but came back with a status code we want to
    /// classify (404 / 403 / 412 / 429 / 5xx). Carries the status so the frontend can
    /// decide UX without re-parsing the message.
    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    /// Network-level failure (connect / timeout / dns / tls) — distinct from `Http`
    /// because the request never produced a status. Frontend should suggest retry and
    /// keep the existing cached events visible. Currently constructed only via the
    /// `classify_reqwest` path (which inspects the inner error and reports `kind() =
    /// "network"`); kept as an explicit variant for future direct construction.
    #[allow(dead_code)]
    #[error("network error: {0}")]
    Network(String),

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

    /// Re-authentication required: the token refresh succeeded but the resulting access
    /// token was still rejected, or CalDAV credentials returned 401. Carries the source
    /// id so the UI can surface "再ログインしてください" against the right account.
    #[error("re-authentication required for source: {0}")]
    AuthRequired(String),

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

/// Discriminant used by the frontend `classifyError` helper. Keep these strings stable —
/// the TS side switches on them. Adding a new kind is fine; renaming an existing one is
/// a breaking change.
impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::MissingCredential(_) => "missing_credential",
            AppError::UnknownSource(_) => "unknown_source",
            AppError::OAuth(_) => "oauth",
            AppError::StateMismatch => "oauth",
            AppError::CallbackTimeout => "oauth_timeout",
            AppError::Http(err) => classify_reqwest(err),
            AppError::HttpStatus { status, .. } => classify_status(*status),
            AppError::Network(_) => "network",
            AppError::Url(_) => "other",
            AppError::Keyring(_) => "keyring",
            AppError::CalDav(_) => "caldav",
            AppError::TokenExpired => "auth_required",
            AppError::NotAuthenticated(_) => "not_authenticated",
            AppError::AuthRequired(_) => "auth_required",
            AppError::Io(_) => "io",
            AppError::Serde(_) => "other",
            AppError::Other(_) => "other",
        }
    }

    /// HTTP status code when the error originated from an HTTP response. `None` for
    /// network/timeout/keyring/oauth-flow failures that don't carry a server status.
    pub fn status(&self) -> Option<u16> {
        match self {
            AppError::Http(err) => err.status().map(|s| s.as_u16()),
            AppError::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Source ID (`ms365_work1` / `google_gws` / `icloud`) when the error is tied to a
    /// specific account. Used by the frontend to prompt re-auth for the right source.
    pub fn source_id(&self) -> Option<&str> {
        match self {
            AppError::NotAuthenticated(s) => Some(s.as_str()),
            AppError::AuthRequired(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Map reqwest errors that never reached a server (connect/timeout/redirect) to the
/// network bucket so the UI can suggest retry. Once the server replied, even with a 5xx,
/// the response has a status — we still call those `http` since the server is reachable.
fn classify_reqwest(err: &reqwest::Error) -> &'static str {
    if err.status().is_some() {
        return classify_status(err.status().unwrap().as_u16());
    }
    if err.is_timeout() || err.is_connect() {
        return "network";
    }
    "http"
}

fn classify_status(status: u16) -> &'static str {
    match status {
        401 => "auth_required",
        403 => "permission",
        404 => "not_found",
        409 | 412 => "conflict",
        429 => "rate_limit",
        500..=599 => "server",
        _ => "http",
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Frontend expects a structured payload. Keys are stable across versions; new
        // optional fields can be added without breaking existing consumers.
        let mut st = s.serialize_struct("AppError", 4)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        match self.status() {
            Some(code) => st.serialize_field("status", &code)?,
            None => st.serialize_field("status", &Option::<u16>::None)?,
        }
        match self.source_id() {
            Some(id) => st.serialize_field("sourceId", id)?,
            None => st.serialize_field("sourceId", &Option::<&str>::None)?,
        }
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
