use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub const MAX_EMBEDDING_INPUTS: usize = 96;
pub const MAX_EMBEDDING_INPUT_CHARS: usize = 700;
pub const MAX_COMBINED_EMBEDDING_CHARS: usize = 24_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticContractError {
    pub code: &'static str,
    pub message: String,
}

impl SemanticContractError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for SemanticContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for SemanticContractError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingInputKind {
    Title,
    Heading,
    Summary,
    Sentence,
    Entity,
    Keyword,
    UrlSignal,
    Document,
    Query,
}

impl EmbeddingInputKind {
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Heading => "heading",
            Self::Summary => "summary",
            Self::Sentence => "sentence",
            Self::Entity => "entity",
            Self::Keyword => "keyword",
            Self::UrlSignal => "url_signal",
            Self::Document => "document",
            Self::Query => "query",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingInput {
    pub kind: EmbeddingInputKind,
    pub ordinal: u16,
    pub text: String,
    pub weight: f32,
}

impl EmbeddingInput {
    pub fn validate(&self) -> Result<(), SemanticContractError> {
        let character_count = self.text.chars().count();
        if !(1..=MAX_EMBEDDING_INPUT_CHARS).contains(&character_count) {
            return Err(SemanticContractError::new(
                "invalid_embedding_input_text",
                format!(
                    "embedding input text must contain 1 to {MAX_EMBEDDING_INPUT_CHARS} characters"
                ),
            ));
        }
        if !self.weight.is_finite() || !(0.1..=2.0).contains(&self.weight) {
            return Err(SemanticContractError::new(
                "invalid_embedding_input_weight",
                "embedding input weight must be finite and between 0.1 and 2.0",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTextViews {
    pub document_text: String,
    pub title: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    pub embedding_inputs: Vec<EmbeddingInput>,
}

impl SemanticTextViews {
    pub fn validate(&self) -> Result<(), SemanticContractError> {
        if self.document_text.trim().is_empty() {
            return Err(SemanticContractError::new(
                "empty_document_text",
                "document_text must not be empty",
            ));
        }
        if self.summary.trim().is_empty() {
            return Err(SemanticContractError::new(
                "empty_summary",
                "summary must not be empty",
            ));
        }
        if self.embedding_inputs.is_empty()
            || self.embedding_inputs.len() > MAX_EMBEDDING_INPUTS
        {
            return Err(SemanticContractError::new(
                "invalid_embedding_inputs",
                format!(
                    "semantic views must contain 1 to {MAX_EMBEDDING_INPUTS} embedding inputs"
                ),
            ));
        }
        for (expected_ordinal, input) in self.embedding_inputs.iter().enumerate() {
            input.validate()?;
            if usize::from(input.ordinal) != expected_ordinal {
                return Err(SemanticContractError::new(
                    "invalid_embedding_input_ordinal",
                    "embedding input ordinals must be contiguous and start at zero",
                ));
            }
        }
        Ok(())
    }

    pub fn combined_embedding_text(&self) -> String {
        let mut output = String::new();
        for input in &self.embedding_inputs {
            let line = format!("{}: {}\n", input.kind.wire_name(), input.text);
            if output.chars().count() + line.chars().count() > MAX_COMBINED_EMBEDDING_CHARS {
                break;
            }
            output.push_str(&line);
        }
        output.trim_end().to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryTextViews {
    pub query_text: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    pub embedding_inputs: Vec<EmbeddingInput>,
}

impl QueryTextViews {
    pub fn validate(&self) -> Result<(), SemanticContractError> {
        let character_count = self.query_text.chars().count();
        if !(3..=2_000).contains(&character_count) {
            return Err(SemanticContractError::new(
                "invalid_query_text",
                "query_text must contain 3 to 2,000 characters",
            ));
        }
        if self.embedding_inputs.is_empty()
            || self.embedding_inputs.len() > MAX_EMBEDDING_INPUTS
        {
            return Err(SemanticContractError::new(
                "invalid_embedding_inputs",
                format!(
                    "query views must contain 1 to {MAX_EMBEDDING_INPUTS} embedding inputs"
                ),
            ));
        }
        for (expected_ordinal, input) in self.embedding_inputs.iter().enumerate() {
            input.validate()?;
            if usize::from(input.ordinal) != expected_ordinal {
                return Err(SemanticContractError::new(
                    "invalid_embedding_input_ordinal",
                    "embedding input ordinals must be contiguous and start at zero",
                ));
            }
        }
        if self.embedding_inputs[0].kind != EmbeddingInputKind::Query {
            return Err(SemanticContractError::new(
                "invalid_query_embedding_inputs",
                "the first query embedding input must preserve the complete query text",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentence_input() -> EmbeddingInput {
        EmbeddingInput {
            kind: EmbeddingInputKind::Sentence,
            ordinal: 0,
            text: "A complete sentence about a renewable energy launch.".into(),
            weight: 1.0,
        }
    }

    #[test]
    fn wire_kind_is_snake_case() {
        assert_eq!(
            serde_json::to_string(&EmbeddingInputKind::UrlSignal).unwrap(),
            "\"url_signal\""
        );
    }

    #[test]
    fn semantic_views_require_ordered_inputs() {
        let mut input = sentence_input();
        input.ordinal = 2;
        let views = SemanticTextViews {
            document_text: "A page about renewable energy.".into(),
            title: None,
            summary: "A page about renewable energy.".into(),
            keywords: vec!["renewable".into(), "energy".into()],
            entities: Vec::new(),
            embedding_inputs: vec![input],
        };
        assert!(views.validate().is_err());
    }

    #[test]
    fn combined_text_retains_view_labels() {
        let views = SemanticTextViews {
            document_text: "A page about renewable energy.".into(),
            title: None,
            summary: "A page about renewable energy.".into(),
            keywords: Vec::new(),
            entities: Vec::new(),
            embedding_inputs: vec![sentence_input()],
        };
        views.validate().unwrap();
        assert_eq!(
            views.combined_embedding_text(),
            "sentence: A complete sentence about a renewable energy launch."
        );
    }

    #[test]
    fn query_views_preserve_full_query_first() {
        let query_text = "Notify me when Acme launches renewable energy tools.";
        let views = QueryTextViews {
            query_text: query_text.into(),
            keywords: vec!["renewable".into(), "energy".into()],
            entities: vec!["Acme".into()],
            embedding_inputs: vec![EmbeddingInput {
                kind: EmbeddingInputKind::Query,
                ordinal: 0,
                text: query_text.into(),
                weight: 1.2,
            }],
        };
        views.validate().unwrap();
    }

    #[test]
    fn non_finite_weights_are_rejected() {
        let mut input = sentence_input();
        input.weight = f32::NAN;
        assert!(input.validate().is_err());
    }
}
