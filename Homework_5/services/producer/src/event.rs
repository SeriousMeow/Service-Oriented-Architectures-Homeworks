use apache_avro::types::Value as AvroValue;
use apache_avro::Schema;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    ViewStarted,
    ViewFinished,
    ViewPaused,
    ViewResumed,
    Liked,
    Searched,
}

impl EventType {
    pub fn as_avro_enum(&self) -> (u32, &'static str) {
        match self {
            EventType::ViewStarted => (0, "VIEW_STARTED"),
            EventType::ViewFinished => (1, "VIEW_FINISHED"),
            EventType::ViewPaused => (2, "VIEW_PAUSED"),
            EventType::ViewResumed => (3, "VIEW_RESUMED"),
            EventType::Liked => (4, "LIKED"),
            EventType::Searched => (5, "SEARCHED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceType {
    Mobile,
    Desktop,
    #[serde(rename = "TV")]
    Tv,
    Tablet,
}

impl DeviceType {
    pub fn as_avro_enum(&self) -> (u32, &'static str) {
        match self {
            DeviceType::Mobile => (0, "MOBILE"),
            DeviceType::Desktop => (1, "DESKTOP"),
            DeviceType::Tv => (2, "TV"),
            DeviceType::Tablet => (3, "TABLET"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieEventPayload {
    pub event_id: Uuid,
    pub user_id: String,
    pub movie_id: String,
    pub event_type: EventType,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub timestamp: DateTime<Utc>,
    pub device_type: DeviceType,
    pub session_id: String,
    pub progress_seconds: i32,
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Ts {
        Str(String),
        Millis(i64),
    }
    match Ts::deserialize(deserializer)? {
        Ts::Str(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.with_timezone(&Utc));
            }
            let ms: i64 = s.parse().map_err(serde::de::Error::custom)?;
            Utc
                .timestamp_millis_opt(ms)
                .single()
                .ok_or_else(|| serde::de::Error::custom("timestamp millis out of range"))
        }
        Ts::Millis(ms) => Utc
            .timestamp_millis_opt(ms)
            .single()
            .ok_or_else(|| serde::de::Error::custom("timestamp millis out of range")),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("user_id must be non-empty")]
    UserId,
    #[error("movie_id must be non-empty")]
    MovieId,
    #[error("session_id must be non-empty")]
    SessionId,
    #[error("progress_seconds must be >= 0")]
    Progress,
}

impl MovieEventPayload {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.user_id.trim().is_empty() {
            return Err(ValidationError::UserId);
        }
        if self.movie_id.trim().is_empty() {
            return Err(ValidationError::MovieId);
        }
        if self.session_id.trim().is_empty() {
            return Err(ValidationError::SessionId);
        }
        if self.progress_seconds < 0 {
            return Err(ValidationError::Progress);
        }
        Ok(())
    }

    pub fn to_avro_value(&self) -> AvroValue {
        let (eti, ets) = self.event_type.as_avro_enum();
        let (dti, dts) = self.device_type.as_avro_enum();
        let ts_millis = self.timestamp.timestamp_millis();
        AvroValue::Record(vec![
            ("event_id".into(), AvroValue::String(self.event_id.to_string())),
            ("user_id".into(), AvroValue::String(self.user_id.clone())),
            ("movie_id".into(), AvroValue::String(self.movie_id.clone())),
            (
                "event_type".into(),
                AvroValue::Enum(eti, ets.into()),
            ),
            (
                "timestamp".into(),
                AvroValue::TimestampMillis(ts_millis),
            ),
            (
                "device_type".into(),
                AvroValue::Enum(dti, dts.into()),
            ),
            ("session_id".into(), AvroValue::String(self.session_id.clone())),
            (
                "progress_seconds".into(),
                AvroValue::Int(self.progress_seconds),
            ),
        ])
    }

    pub fn encode_avro(&self, root: &Schema) -> Result<Vec<u8>, apache_avro::Error> {
        let v = self.to_avro_value();
        apache_avro::to_avro_datum(root, v)
    }
}
