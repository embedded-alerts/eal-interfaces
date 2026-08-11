//! Semantic ingestion, embedding-space, matching, and delivery contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

pub const CONTRACT_VERSION: &str = "2026-08-10";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

fn require_text(value: &str, field: &str, max_bytes: usize) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError(format!("{field} must not be empty")));
    }
    if value.len() > max_bytes {
        return Err(ValidationError(format!(
            "{field} exceeds {max_bytes} bytes"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Rss,
    Atom,
    JsonFeed,
    SearchQuery,
    Sitemap,
    WebPage,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateSource {
    pub kind: SourceKind,
    pub name: String,
    pub url: String,
    #[serde(default = "default_poll_interval_seconds")]
    pub poll_interval_seconds: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub public_config: Value,
    /// Opaque secret-manager reference accepted on writes and never returned by read APIs.
    pub credential_reference: Option<String>,
}

impl CreateSource {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_text(&self.name, "name", 256)?;
        require_text(&self.url, "url", 8_192)?;
        if !(self.url.starts_with("https://") || self.url.starts_with("http://")) {
            return Err(ValidationError("url must use http or https".into()));
        }
        if !(60..=604_800).contains(&self.poll_interval_seconds) {
            return Err(ValidationError(
                "poll_interval_seconds must be between 60 and 604800".into(),
            ));
        }
        if let Some(reference) = &self.credential_reference {
            require_text(reference, "credential_reference", 2_048)?;
        }
        Ok(())
    }
}

const fn default_poll_interval_seconds() -> u32 {
    900
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingNormalization {
    None,
    L2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingSpace {
    pub id: Uuid,
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub dimensions: u16,
    pub normalization: EmbeddingNormalization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingVector {
    pub space: EmbeddingSpace,
    pub values: Vec<f32>,
    pub generated_at: DateTime<Utc>,
}

impl EmbeddingVector {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_text(&self.space.provider, "provider", 128)?;
        require_text(&self.space.model, "model", 256)?;
        require_text(&self.space.model_version, "model_version", 256)?;
        if self.space.dimensions == 0 || self.space.dimensions > 32_768 {
            return Err(ValidationError(
                "dimensions must be between 1 and 32768".into(),
            ));
        }
        if self.values.len() != usize::from(self.space.dimensions) {
            return Err(ValidationError("embedding dimension mismatch".into()));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError(
                "embedding values must all be finite".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertRuleRevisionSpec {
    pub query_text: String,
    pub embedding_space_id: Option<Uuid>,
    pub semantic_threshold: f32,
    pub semantic_weight: f32,
    pub lexical_weight: f32,
    #[serde(default)]
    pub required_terms: Vec<String>,
    #[serde(default)]
    pub excluded_terms: Vec<String>,
    #[serde(default)]
    pub source_ids: Vec<Uuid>,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u32,
}

impl AlertRuleRevisionSpec {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_text(&self.query_text, "query_text", 16_384)?;
        for (field, value) in [
            ("semantic_threshold", self.semantic_threshold),
            ("semantic_weight", self.semantic_weight),
            ("lexical_weight", self.lexical_weight),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ValidationError(format!("{field} must be between 0 and 1")));
            }
        }
        if (self.semantic_weight + self.lexical_weight - 1.0).abs() > 0.000_1 {
            return Err(ValidationError(
                "semantic_weight and lexical_weight must sum to 1".into(),
            ));
        }
        if self.cooldown_seconds > 2_592_000 {
            return Err(ValidationError(
                "cooldown_seconds must not exceed 30 days".into(),
            ));
        }
        for term in self.required_terms.iter().chain(&self.excluded_terms) {
            require_text(term, "term", 256)?;
        }
        Ok(())
    }
}

const fn default_cooldown_seconds() -> u32 {
    3_600
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    Candidate,
    Suppressed,
    Queued,
    Delivered,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchCandidate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub identity_key: String,
    pub alert_rule_id: Uuid,
    pub alert_rule_revision_id: Uuid,
    pub source_document_id: Uuid,
    pub source_revision_id: Uuid,
    pub embedding_space_id: Option<Uuid>,
    pub semantic_score: Option<f32>,
    pub lexical_score: f32,
    pub total_score: f32,
    #[serde(default)]
    pub explanation: Value,
    pub status: MatchStatus,
    pub matched_at: DateTime<Utc>,
    pub suppressed_until: Option<DateTime<Utc>>,
}

impl MatchCandidate {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.identity_key.len() != 64
            || !self
                .identity_key
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ValidationError(
                "identity_key must be a SHA-256 hexadecimal digest".into(),
            ));
        }
        for score in [
            self.semantic_score,
            Some(self.lexical_score),
            Some(self.total_score),
        ] {
            if score.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
                return Err(ValidationError("scores must be between 0 and 1".into()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryAttemptStatus {
    Pending,
    Delivering,
    Succeeded,
    RetryScheduled,
    DeadLettered,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeliveryAttempt {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub match_id: Uuid,
    pub delivery_target_id: Uuid,
    pub idempotency_key: String,
    pub attempt: u32,
    pub status: DeliveryAttemptStatus,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub provider_reference: Option<String>,
    pub response_status: Option<u16>,
    pub error_class: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_sources() {
        let source = CreateSource {
            kind: SourceKind::WebPage,
            name: "unsafe".into(),
            url: "file:///etc/passwd".into(),
            poll_interval_seconds: 300,
            enabled: true,
            public_config: Value::Null,
            credential_reference: None,
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn rejects_cross_dimension_vectors() {
        let vector = EmbeddingVector {
            space: EmbeddingSpace {
                id: Uuid::new_v4(),
                provider: "local".into(),
                model: "mini".into(),
                model_version: "1".into(),
                dimensions: 3,
                normalization: EmbeddingNormalization::L2,
            },
            values: vec![0.1, 0.2],
            generated_at: Utc::now(),
        };
        assert!(vector.validate().is_err());
    }

    #[test]
    fn wire_status_is_stable() {
        let value = serde_json::to_string(&DeliveryAttemptStatus::RetryScheduled).unwrap();
        assert_eq!(value, "\"retry_scheduled\"");
    }
}
