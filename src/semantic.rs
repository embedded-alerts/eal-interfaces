//! Semantic ingestion, embedding-space, matching, delivery, and crawl-policy contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, fmt};
use url::{Host, Url};
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

impl SourceKind {
    const fn source_url_must_match_policy(self) -> bool {
        !matches!(self, Self::SearchQuery)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMode {
    Manual,
    Sitemap,
    Rss,
    Atom,
    JsonFeed,
    SearchProviderCandidates,
    Link,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicPageFetchPolicy {
    pub allowed_hosts: Vec<String>,
    #[serde(default = "default_allowed_path_prefixes")]
    pub allowed_path_prefixes: Vec<String>,
    #[serde(default)]
    pub include_subdomains: bool,
    #[serde(default = "default_discovery_modes")]
    pub discovery_modes: Vec<DiscoveryMode>,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default = "default_max_pages_per_run")]
    pub max_pages_per_run: u32,
    #[serde(default = "default_max_concurrent_requests_per_host")]
    pub max_concurrent_requests_per_host: u8,
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u16,
    #[serde(default = "default_max_response_bytes")]
    pub max_response_bytes: u64,
    #[serde(default = "default_allowed_content_types")]
    pub allowed_content_types: Vec<String>,
    #[serde(default = "default_true")]
    pub obey_robots: bool,
}

impl PublicPageFetchPolicy {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.allowed_hosts.is_empty() || self.allowed_hosts.len() > 64 {
            return Err(ValidationError(
                "allowed_hosts must contain between 1 and 64 exact public hosts".into(),
            ));
        }
        let mut normalized_hosts = HashSet::with_capacity(self.allowed_hosts.len());
        for host in &self.allowed_hosts {
            let normalized = validate_public_host(host)?;
            if !normalized_hosts.insert(normalized) {
                return Err(ValidationError(
                    "allowed_hosts must not contain duplicates".into(),
                ));
            }
        }

        if self.allowed_path_prefixes.is_empty() || self.allowed_path_prefixes.len() > 128 {
            return Err(ValidationError(
                "allowed_path_prefixes must contain between 1 and 128 entries".into(),
            ));
        }
        let mut path_prefixes = HashSet::with_capacity(self.allowed_path_prefixes.len());
        for prefix in &self.allowed_path_prefixes {
            if !prefix.starts_with('/')
                || prefix.contains('#')
                || prefix.contains('?')
                || prefix.split('/').any(|segment| segment == "..")
            {
                return Err(ValidationError(format!(
                    "invalid allowed path prefix: {prefix}"
                )));
            }
            if !path_prefixes.insert(prefix) {
                return Err(ValidationError(
                    "allowed_path_prefixes must not contain duplicates".into(),
                ));
            }
        }

        if self.discovery_modes.is_empty() {
            return Err(ValidationError(
                "discovery_modes must contain at least one mode".into(),
            ));
        }
        let mut discovery_modes = HashSet::with_capacity(self.discovery_modes.len());
        if self
            .discovery_modes
            .iter()
            .any(|mode| !discovery_modes.insert(*mode))
        {
            return Err(ValidationError(
                "discovery_modes must not contain duplicates".into(),
            ));
        }
        if self.max_depth > 16 {
            return Err(ValidationError("max_depth must not exceed 16".into()));
        }
        if !(1..=10_000).contains(&self.max_pages_per_run) {
            return Err(ValidationError(
                "max_pages_per_run must be between 1 and 10000".into(),
            ));
        }
        if !(1..=8).contains(&self.max_concurrent_requests_per_host) {
            return Err(ValidationError(
                "max_concurrent_requests_per_host must be between 1 and 8".into(),
            ));
        }
        if !(1..=60).contains(&self.request_timeout_seconds) {
            return Err(ValidationError(
                "request_timeout_seconds must be between 1 and 60".into(),
            ));
        }
        if !(1_024..=20_000_000).contains(&self.max_response_bytes) {
            return Err(ValidationError(
                "max_response_bytes must be between 1024 and 20000000".into(),
            ));
        }
        if self.allowed_content_types.is_empty() || self.allowed_content_types.len() > 32 {
            return Err(ValidationError(
                "allowed_content_types must contain between 1 and 32 exact media types".into(),
            ));
        }
        let mut content_types = HashSet::with_capacity(self.allowed_content_types.len());
        for content_type in &self.allowed_content_types {
            let normalized = content_type.trim().to_ascii_lowercase();
            if normalized.is_empty()
                || normalized.contains('*')
                || normalized.contains(';')
                || !normalized.contains('/')
            {
                return Err(ValidationError(format!(
                    "invalid exact content type: {content_type}"
                )));
            }
            if !content_types.insert(normalized) {
                return Err(ValidationError(
                    "allowed_content_types must not contain duplicates".into(),
                ));
            }
        }
        if !self.obey_robots {
            return Err(ValidationError(
                "obey_robots cannot be disabled for public-page indexing".into(),
            ));
        }
        Ok(())
    }

    pub fn validate_source_url(&self, raw_url: &str) -> Result<(), ValidationError> {
        self.validate()?;
        let url = validate_public_url(raw_url)?;
        let host = match url.host() {
            Some(Host::Domain(host)) => host.to_ascii_lowercase(),
            Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {
                return Err(ValidationError(
                    "literal IP addresses are forbidden for public sources".into(),
                ));
            }
            None => return Err(ValidationError("source URL must include a host".into())),
        };
        if !self.host_is_allowed(&host) {
            return Err(ValidationError(format!(
                "source host {host} is outside allowed_hosts"
            )));
        }
        if !self
            .allowed_path_prefixes
            .iter()
            .any(|prefix| path_is_allowed(url.path(), prefix))
        {
            return Err(ValidationError(format!(
                "source path {} is outside allowed_path_prefixes",
                url.path()
            )));
        }
        Ok(())
    }

    pub fn host_is_allowed(&self, raw_host: &str) -> bool {
        let host = raw_host.trim().trim_end_matches('.').to_ascii_lowercase();
        self.allowed_hosts.iter().any(|allowed| {
            let allowed = allowed.trim().trim_end_matches('.').to_ascii_lowercase();
            host == allowed
                || (self.include_subdomains
                    && host
                        .strip_suffix(&allowed)
                        .is_some_and(|prefix| prefix.ends_with('.') && prefix.len() > 1))
        })
    }
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
    /// Required for every source. Search providers may propose URLs, but every candidate
    /// still has to satisfy this exact host/path and crawl-budget policy.
    #[serde(default)]
    pub fetch_policy: Option<PublicPageFetchPolicy>,
}

impl CreateSource {
    pub fn validate(&self) -> Result<(), ValidationError> {
        require_text(&self.name, "name", 256)?;
        let source_url = validate_public_url(&self.url)?;
        if !(60..=604_800).contains(&self.poll_interval_seconds) {
            return Err(ValidationError(
                "poll_interval_seconds must be between 60 and 604800".into(),
            ));
        }
        if !self.public_config.is_object() {
            return Err(ValidationError("public_config must be an object".into()));
        }
        if let Some(reference) = &self.credential_reference {
            require_text(reference, "credential_reference", 2_048)?;
        }

        let policy = self.fetch_policy.as_ref().ok_or_else(|| {
            ValidationError("fetch_policy is required for every source connector".into())
        })?;
        policy.validate()?;
        if self.kind.source_url_must_match_policy() {
            policy.validate_source_url(source_url.as_str())?;
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

fn default_allowed_path_prefixes() -> Vec<String> {
    vec!["/".into()]
}

fn default_discovery_modes() -> Vec<DiscoveryMode> {
    vec![DiscoveryMode::Sitemap, DiscoveryMode::Rss]
}

const fn default_max_depth() -> u8 {
    3
}

const fn default_max_pages_per_run() -> u32 {
    1_000
}

const fn default_max_concurrent_requests_per_host() -> u8 {
    2
}

const fn default_request_timeout_seconds() -> u16 {
    20
}

const fn default_max_response_bytes() -> u64 {
    5_000_000
}

fn default_allowed_content_types() -> Vec<String> {
    vec![
        "text/html".into(),
        "application/xhtml+xml".into(),
        "application/rss+xml".into(),
        "application/atom+xml".into(),
        "application/feed+json".into(),
    ]
}

fn validate_public_url(raw_url: &str) -> Result<Url, ValidationError> {
    let url = Url::parse(raw_url)
        .map_err(|error| ValidationError(format!("invalid absolute URL: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ValidationError("url must use http or https".into()));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ValidationError("URL credentials are forbidden".into()));
    }
    if url.fragment().is_some() {
        return Err(ValidationError(
            "source URLs must not contain fragments".into(),
        ));
    }
    if let Some(port) = url.port() {
        let is_default = matches!((url.scheme(), port), ("http", 80) | ("https", 443));
        if !is_default {
            return Err(ValidationError(
                "public source URLs must use the default HTTP or HTTPS port".into(),
            ));
        }
    }
    match url.host() {
        Some(Host::Domain(host)) => {
            validate_public_host(host)?;
        }
        Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {
            return Err(ValidationError(
                "literal IP addresses are forbidden for public sources".into(),
            ));
        }
        None => return Err(ValidationError("url must include a public host".into())),
    }
    Ok(url)
}

fn validate_public_host(raw_host: &str) -> Result<String, ValidationError> {
    let host = raw_host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || !host.contains('.')
        || host.contains('*')
        || host.contains('/')
        || host.contains(':')
        || host.contains('@')
        || host.chars().any(char::is_whitespace)
    {
        return Err(ValidationError(format!(
            "allowed host must be an exact public DNS name: {raw_host}"
        )));
    }
    Ok(host)
}

fn path_is_allowed(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }
    let prefix = prefix.trim_end_matches('/');
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remaining| remaining.starts_with('/'))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlQueueStatus {
    Pending,
    Leased,
    Fetched,
    Unchanged,
    Blocked,
    Failed,
    DeadLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrawlCandidate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source_id: Uuid,
    pub candidate_url: String,
    pub canonical_url: String,
    pub discovered_by: DiscoveryMode,
    pub depth: u8,
    pub status: CrawlQueueStatus,
    pub priority: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub attempt_count: u32,
    pub last_error_class: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

    fn public_policy() -> PublicPageFetchPolicy {
        PublicPageFetchPolicy {
            allowed_hosts: vec!["blog.rust-lang.org".into()],
            allowed_path_prefixes: vec!["/inside".into()],
            include_subdomains: false,
            discovery_modes: vec![DiscoveryMode::Sitemap, DiscoveryMode::Rss],
            max_depth: 3,
            max_pages_per_run: 1_000,
            max_concurrent_requests_per_host: 2,
            request_timeout_seconds: 20,
            max_response_bytes: 5_000_000,
            allowed_content_types: default_allowed_content_types(),
            obey_robots: true,
        }
    }

    #[test]
    fn rejects_non_http_sources() {
        let source = CreateSource {
            kind: SourceKind::WebPage,
            name: "unsafe".into(),
            url: "file:///etc/passwd".into(),
            poll_interval_seconds: 300,
            enabled: true,
            public_config: serde_json::json!({}),
            credential_reference: None,
            fetch_policy: Some(public_policy()),
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn public_sources_require_exact_domain_policy() {
        let source = CreateSource {
            kind: SourceKind::WebPage,
            name: "Rust".into(),
            url: "https://blog.rust-lang.org/inside/release".into(),
            poll_interval_seconds: 300,
            enabled: true,
            public_config: serde_json::json!({}),
            credential_reference: None,
            fetch_policy: Some(public_policy()),
        };
        assert!(source.validate().is_ok());

        let mut unsafe_source = source.clone();
        unsafe_source.url = "https://example.com/inside/release".into();
        assert!(unsafe_source.validate().is_err());

        let mut wildcard_policy = public_policy();
        wildcard_policy.allowed_hosts = vec!["*.rust-lang.org".into()];
        unsafe_source.url = "https://blog.rust-lang.org/inside/release".into();
        unsafe_source.fetch_policy = Some(wildcard_policy);
        assert!(unsafe_source.validate().is_err());
    }

    #[test]
    fn path_prefixes_are_segment_aware() {
        let policy = public_policy();
        assert!(policy
            .validate_source_url("https://blog.rust-lang.org/inside/item")
            .is_ok());
        assert!(policy
            .validate_source_url("https://blog.rust-lang.org/inside-item")
            .is_err());
    }

    #[test]
    fn search_provider_candidates_still_require_target_domains() {
        let source = CreateSource {
            kind: SourceKind::SearchQuery,
            name: "Provider query".into(),
            url: "https://search.example.com/query?q=rust".into(),
            poll_interval_seconds: 300,
            enabled: true,
            public_config: serde_json::json!({}),
            credential_reference: Some("secret://search-provider".into()),
            fetch_policy: Some(public_policy()),
        };
        assert!(source.validate().is_ok());

        let mut missing_policy = source;
        missing_policy.fetch_policy = None;
        assert!(missing_policy.validate().is_err());
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
