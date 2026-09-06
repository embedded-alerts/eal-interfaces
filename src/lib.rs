use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
        require_non_empty("title", &self.title, 256)?;
        require_max_len("summary", &self.summary, 4_000)?;
        require_non_empty("query", &self.query, 32_000)?;
        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(ValidationError(
                "threshold must be between 0 and 1".into(),
            ));
        }
        require_non_empty("delivery_channel", &self.delivery_channel, 256)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMode {
    Manual,
    Sitemap,
    Rss,
    Atom,
    ExternalIndexCandidates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePolicy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub root_url: String,
    pub allowed_hosts: Vec<String>,
    pub allowed_path_prefixes: Vec<String>,
    pub include_subdomains: bool,
    pub discovery_modes: Vec<DiscoveryMode>,
    pub crawl_interval_seconds: u32,
    pub max_depth: u8,
    pub max_pages_per_run: u32,
    pub obey_robots: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourcePolicy {
    pub name: String,
    pub root_url: String,
    pub allowed_hosts: Vec<String>,
    #[serde(default = "default_path_prefixes")]
    pub allowed_path_prefixes: Vec<String>,
    #[serde(default)]
    pub include_subdomains: bool,
    #[serde(default = "default_discovery_modes")]
    pub discovery_modes: Vec<DiscoveryMode>,
    #[serde(default = "default_crawl_interval_seconds")]
    pub crawl_interval_seconds: u32,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default = "default_max_pages_per_run")]
    pub max_pages_per_run: u32,
    #[serde(default = "default_true")]
    pub obey_robots: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl CreateSourcePolicy {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_non_empty("name", &self.name, 256)?;
        require_http_url_shape("root_url", &self.root_url)?;

        if self.allowed_hosts.is_empty() {
            return Err(ValidationError(
                "allowed_hosts must contain at least one exact host".into(),
            ));
        }
        if self.allowed_hosts.len() > 64 {
            return Err(ValidationError(
                "allowed_hosts must contain at most 64 hosts".into(),
            ));
        }
        for host in &self.allowed_hosts {
            validate_host(host)?;
        }

        if self.allowed_path_prefixes.is_empty() {
            return Err(ValidationError(
                "allowed_path_prefixes must contain at least one path".into(),
            ));
        }
        if self.allowed_path_prefixes.len() > 128 {
            return Err(ValidationError(
                "allowed_path_prefixes must contain at most 128 paths".into(),
            ));
        }
        for path in &self.allowed_path_prefixes {
            if !path.starts_with('/') || path.contains('#') {
                return Err(ValidationError(format!(
                    "allowed path prefix must start with '/' and omit fragments: {path}"
                )));
            }
        }

        if self.discovery_modes.is_empty() {
            return Err(ValidationError(
                "discovery_modes must contain at least one mode".into(),
            ));
        }
        if !(60..=604_800).contains(&self.crawl_interval_seconds) {
            return Err(ValidationError(
                "crawl_interval_seconds must be between 60 and 604800".into(),
            ));
        }
        if self.max_depth > 16 {
            return Err(ValidationError("max_depth must be at most 16".into()));
        }
        if !(1..=10_000).contains(&self.max_pages_per_run) {
            return Err(ValidationError(
                "max_pages_per_run must be between 1 and 10000".into(),
            ));
        }
        if !self.obey_robots {
            return Err(ValidationError(
                "obey_robots cannot be disabled for public-page indexing".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorNormalization {
    None,
    L2,
    UnitLength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingPayload {
    pub model: String,
    pub model_version: String,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
    pub values: Vec<f32>,
}

impl EmbeddingPayload {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_non_empty("model", &self.model, 256)?;
        require_non_empty("model_version", &self.model_version, 256)?;
        if self.dimensions == 0 || self.dimensions > 65_535 {
            return Err(ValidationError(
                "dimensions must be between 1 and 65535".into(),
            ));
        }
        if self.values.len() != self.dimensions as usize {
            return Err(ValidationError(format!(
                "embedding length {} does not match dimensions {}",
                self.values.len(),
                self.dimensions
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError(
                "embedding values must all be finite".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIngestRequest {
    pub source_id: Uuid,
    pub url: String,
    pub final_url: String,
    pub title: Option<String>,
    pub content_text: String,
    pub content_type: String,
    pub http_status: u16,
    pub published_at: Option<DateTime<Utc>>,
    pub fetched_at: DateTime<Utc>,
    pub embedding: EmbeddingPayload,
}

impl PageIngestRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_http_url_shape("url", &self.url)?;
        require_http_url_shape("final_url", &self.final_url)?;
        require_non_empty("content_text", &self.content_text, 5_000_000)?;
        require_non_empty("content_type", &self.content_type, 256)?;
        if !(200..=399).contains(&self.http_status) {
            return Err(ValidationError(
                "http_status must represent a successful or redirect response".into(),
            ));
        }
        self.embedding.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRevision {
    pub page_id: Uuid,
    pub revision_id: Uuid,
    pub embedding_id: Uuid,
    pub source_id: Uuid,
    pub tenant_id: Uuid,
    pub canonical_url: String,
    pub content_sha256: String,
    pub changed: bool,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCursor {
    pub distance: f64,
    pub embedding_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSearchRequest {
    pub embedding: EmbeddingPayload,
    #[serde(default = "default_min_similarity")]
    pub min_similarity: f32,
    #[serde(default = "default_search_limit")]
    pub limit: u16,
    pub cursor: Option<SearchCursor>,
    #[serde(default)]
    pub source_ids: Vec<Uuid>,
}

impl EmbeddingSearchRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.embedding.validate()?;
        if !(0.0..=1.0).contains(&self.min_similarity) {
            return Err(ValidationError(
                "min_similarity must be between 0 and 1".into(),
            ));
        }
        if !(1..=200).contains(&self.limit) {
            return Err(ValidationError("limit must be between 1 and 200".into()));
        }
        if self.source_ids.len() > 100 {
            return Err(ValidationError(
                "source_ids must contain at most 100 IDs".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSearchHit {
    pub embedding_id: Uuid,
    pub revision_id: Uuid,
    pub page_id: Uuid,
    pub source_id: Uuid,
    pub canonical_url: String,
    pub title: Option<String>,
    pub excerpt: String,
    pub similarity: f64,
    pub distance: f64,
    pub model: String,
    pub model_version: String,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSearchResponse {
    pub hits: Vec<EmbeddingSearchHit>,
    pub next_cursor: Option<SearchCursor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchCandidateStatus {
    Pending,
    Suppressed,
    Approved,
    Rejected,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCandidate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub alert_rule_id: Uuid,
    pub revision_id: Uuid,
    pub embedding_id: Uuid,
    pub canonical_match_key: String,
    pub similarity: f64,
    pub threshold: f64,
    pub status: MatchCandidateStatus,
    pub score_explanation: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

fn require_non_empty(name: &str, value: &str, max_len: usize) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError(format!("{name} must not be empty")));
    }
    require_max_len(name, value, max_len)
}

fn require_max_len(name: &str, value: &str, max_len: usize) -> Result<(), ValidationError> {
    if value.len() > max_len {
        return Err(ValidationError(format!(
            "{name} exceeds {max_len} bytes"
        )));
    }
    Ok(())
}

fn require_http_url_shape(name: &str, value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err(ValidationError(format!(
            "{name} must be an absolute http or https URL"
        )));
    }
    if trimmed.contains('@') {
        return Err(ValidationError(format!(
            "{name} must not contain URL credentials"
        )));
    }
    Ok(())
}

fn validate_host(value: &str) -> Result<(), ValidationError> {
    let host = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty()
        || host.contains('/')
        || host.contains(':')
        || host.contains('*')
        || host.contains('@')
        || host == "localhost"
        || !host.contains('.')
    {
        return Err(ValidationError(format!(
            "allowed host must be an exact public DNS name: {value}"
        )));
    }
    Ok(())
}

fn default_true() -> bool {
    true
}

fn default_path_prefixes() -> Vec<String> {
    vec!["/".into()]
}

fn default_discovery_modes() -> Vec<DiscoveryMode> {
    vec![DiscoveryMode::Sitemap, DiscoveryMode::Rss]
}

fn default_crawl_interval_seconds() -> u32 {
    900
}

fn default_max_depth() -> u8 {
    3
}

fn default_max_pages_per_run() -> u32 {
    1_000
}

fn default_min_similarity() -> f32 {
    0.78
}

fn default_search_limit() -> u16 {
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_source() -> CreateSourcePolicy {
        CreateSourcePolicy {
            name: "Rust blog".into(),
            root_url: "https://blog.rust-lang.org/".into(),
            allowed_hosts: vec!["blog.rust-lang.org".into()],
            allowed_path_prefixes: vec!["/".into()],
            include_subdomains: false,
            discovery_modes: vec![DiscoveryMode::Sitemap, DiscoveryMode::Rss],
            crawl_interval_seconds: 900,
            max_depth: 3,
            max_pages_per_run: 500,
            obey_robots: true,
            enabled: true,
        }
    }

    #[test]
    fn status_serializes_as_wire_value() {
        let value = serde_json::to_string(&AlertRuleStatus::default()).unwrap();
        assert_eq!(value, serde_json::to_string(&"draft").unwrap());
    }

    #[test]
    fn source_policy_requires_explicit_public_hosts_and_robots() {
        assert!(valid_source().validate().is_ok());

        let mut source = valid_source();
        source.allowed_hosts = vec!["*.example.com".into()];
        assert!(source.validate().is_err());

        let mut source = valid_source();
        source.obey_robots = false;
        assert!(source.validate().is_err());
    }

    #[test]
    fn embedding_payload_cannot_hide_dimension_or_nan_mismatch() {
        let payload = EmbeddingPayload {
            model: "example".into(),
            model_version: "2026-08-01".into(),
            dimensions: 3,
            normalization: VectorNormalization::UnitLength,
            values: vec![0.1, 0.2],
        };
        assert!(payload.validate().is_err());

        let payload = EmbeddingPayload {
            values: vec![0.1, f32::NAN, 0.3],
            ..payload
        };
        assert!(payload.validate().is_err());
    }

    #[test]
    fn search_is_bounded() {
        let request = EmbeddingSearchRequest {
            embedding: EmbeddingPayload {
                model: "example".into(),
                model_version: "v1".into(),
                dimensions: 3,
                normalization: VectorNormalization::L2,
                values: vec![0.1, 0.2, 0.3],
            },
            min_similarity: 0.78,
            limit: 201,
            cursor: None,
            source_ids: vec![],
        };
        assert!(request.validate().is_err());
    }
}
