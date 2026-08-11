//! Canonical Rust, JSON Schema, OpenAPI, AsyncAPI, and PostgreSQL contracts for Embedded Alerts.

pub mod semantic;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertRuleStatus {
    #[default]
    Draft,
    Active,
    Paused,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub query: String,
    pub threshold: f32,
    pub delivery_channel: String,
    pub enabled: bool,
    pub status: AlertRuleStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRule {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub query: String,
    pub threshold: f32,
    pub delivery_channel: String,
    pub enabled: bool,
}

impl CreateAlertRule {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.title.trim().is_empty() {
            return Err(ValidationError("title must not be empty".into()));
        }
        if self.summary.len() > 4_000 {
            return Err(ValidationError("summary exceeds 4000 bytes".into()));
        }
        if self.query.trim().is_empty() {
            return Err(ValidationError("query must not be empty".into()));
        }
        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(ValidationError(
                "threshold must be between 0 and 1".into(),
            ));
        }
        if self.delivery_channel.trim().is_empty() {
            return Err(ValidationError(
                "delivery_channel must not be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn into_record(
        self,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<AlertRule, ValidationError> {
        self.validate()?;
        Ok(AlertRule {
            id,
            title: self.title,
            summary: self.summary,
            query: self.query,
            threshold: self.threshold,
            delivery_channel: self.delivery_channel,
            enabled: self.enabled,
            status: AlertRuleStatus::default(),
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleEvent {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub data: AlertRule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_as_wire_value() {
        let value = serde_json::to_string(&AlertRuleStatus::default()).unwrap();
        assert_eq!(value, "\"draft\"");
    }
}
