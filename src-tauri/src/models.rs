use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSourceId {
    Ms365Work1,
    GoogleGws,
    Icloud,
}

impl CalendarSourceId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ms365_work1" => Some(Self::Ms365Work1),
            "google_gws" => Some(Self::GoogleGws),
            "icloud" => Some(Self::Icloud),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ms365Work1 => "ms365_work1",
            Self::GoogleGws => "google_gws",
            Self::Icloud => "icloud",
        }
    }

    pub fn protocol(&self) -> Protocol {
        match self {
            Self::Ms365Work1 => Protocol::Graph,
            Self::GoogleGws => Protocol::GCal,
            Self::Icloud => Protocol::CalDav,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Graph,
    GCal,
    CalDav,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarMeta {
    pub id: String,
    #[serde(rename = "sourceId")]
    pub source_id: CalendarSourceId,
    pub name: String,
    #[serde(rename = "isPrimary")]
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(rename = "isWritable")]
    pub is_writable: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEvent {
    pub id: String,
    #[serde(rename = "sourceId")]
    pub source_id: CalendarSourceId,
    #[serde(rename = "calendarId")]
    pub calendar_id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    #[serde(rename = "isAllDay")]
    pub is_all_day: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(rename = "isRecurring")]
    pub is_recurring: bool,
    #[serde(rename = "recurringEventId", skip_serializing_if = "Option::is_none")]
    pub recurring_event_id: Option<String>,
    #[serde(rename = "recurrenceRule", skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurringEditScope {
    This,
    ThisAndFollowing,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDraft {
    #[serde(rename = "sourceId")]
    pub source_id: CalendarSourceId,
    #[serde(rename = "calendarId")]
    pub calendar_id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    #[serde(rename = "isAllDay")]
    pub is_all_day: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventUpdateRequest {
    pub draft: EventDraft,
    #[serde(rename = "recurringScope", default, skip_serializing_if = "Option::is_none")]
    pub recurring_scope: Option<RecurringEditScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    #[serde(rename = "sourceId")]
    pub source_id: CalendarSourceId,
    pub connected: bool,
    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
