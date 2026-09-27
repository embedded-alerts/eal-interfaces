use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Warning,
    Error,
}

#[derive(Debug)]
struct Problem {
    severity: Severity,
    path: String,
    message: String,
}

fn main() {
    let mut format = "text";
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--format" {
            format = args.next().as_deref().unwrap_or("text");
        }
    }

    let root = env::current_dir().expect("current directory must be available");
    let (files, problems) = validate_repo(&root);
    let error_count = problems
        .iter()
        .filter(|problem| problem.severity == Severity::Error)
        .count();

    for problem in &problems {
        match format {
            "github" => {
                let level = match problem.severity {
                    Severity::Warning => "warning",
                    Severity::Error => "error",
                };
                println!("::{level} file={}::{}", problem.path, problem.message);
            }
            _ => {
                let level = match problem.severity {
                    Severity::Warning => "warn",
                    Severity::Error => "FAIL",
                };
                println!("  {level:4} {}: {}", problem.path, problem.message);
            }
        }
    }

    if format != "github" {
        println!(
            "\n{} schema(s) checked: {} error(s), {} warning(s)",
            files.len(),
            error_count,
            problems.len().saturating_sub(error_count)
        );
    }

    if error_count > 0 {
        std::process::exit(1);
    }
}

fn validate_repo(root: &Path) -> (Vec<PathBuf>, Vec<Problem>) {
    let files = find_schema_files(root);
    let mut problems = Vec::new();
    let mut ids: BTreeMap<String, String> = BTreeMap::new();

    if files.is_empty() {
        problems.push(Problem {
            severity: Severity::Warning,
            path: String::new(),
            message: "no JSON Schemas found under schema/ or schemas/".into(),
        });
        return (files, problems);
    }

    for path in &files {
        let rel = relative_display(root, path);
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) => {
                problems.push(Problem {
                    severity: Severity::Error,
                    path: rel,
                    message: format!("cannot read schema: {error}"),
                });
                continue;
            }
        };
        let document: Value = match serde_json::from_str(&raw) {
            Ok(document) => document,
            Err(error) => {
                problems.push(Problem {
                    severity: Severity::Error,
                    path: rel,
                    message: format!("does not parse as JSON: {error}"),
                });
                continue;
            }
        };
        let Some(object) = document.as_object() else {
            problems.push(Problem {
                severity: Severity::Error,
                path: rel,
                message: "top level is not a schema object".into(),
            });
            continue;
        };

        if !object.contains_key("$schema") {
            problems.push(Problem {
                severity: Severity::Warning,
                path: rel.clone(),
                message: "no $schema; validators have to guess the dialect".into(),
            });
        }
        match object.get("$id").and_then(Value::as_str) {
            Some(id) => {
                if let Some(previous) = ids.insert(id.to_owned(), rel.clone()) {
                    problems.push(Problem {
                        severity: Severity::Error,
                        path: rel.clone(),
                        message: format!("$id {id:?} is already used by {previous}"),
                    });
                }
            }
            None => problems.push(Problem {
                severity: Severity::Warning,
                path: rel.clone(),
                message: "no $id; consumers cannot reference it stably".into(),
            }),
        }

        validate_schema_node(&document, &rel, path, &document, &mut problems);
    }

    return (files, problems);
}

fn find_schema_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for directory in ["schema", "schemas"] {
        let path = root.join(directory);
        if path.is_dir() {
            walk_directory(&path, &mut files);
        }
    }
    files.sort();
    return files;
}

fn walk_directory(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(
                name.as_ref(),
                "node_modules"
                    | "target"
                    | "build"
                    | "dist"
                    | "vendor"
                    | ".git"
                    | "_build"
                    | "deps"
                    | "obj"
                    | "out"
                    | "__pycache__"
                    | "tmp"
            ) || name.starts_with('.')
            {
                continue;
            }
            walk_directory(&path, files);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("json")
            && path.file_name().and_then(|value| value.to_str()) != Some("index.json")
        {
            files.push(path);
        }
    }
}

fn validate_schema_node(
    node: &Value,
    rel: &str,
    file_path: &Path,
    root_document: &Value,
    problems: &mut Vec<Problem>,
) {
    let Some(object) = node.as_object() else {
        if !node.is_boolean() {
            problems.push(Problem {
                severity: Severity::Error,
                path: rel.into(),
                message: "schema node must be an object or boolean".into(),
            });
        }
        return;
    };

    validate_keyword_shapes(object, rel, problems);
    validate_ref(object, rel, file_path, root_document, problems);

    for keyword in [
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "items",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    ] {
        if let Some(child) = object.get(keyword) {
            validate_schema_node(child, rel, file_path, root_document, problems);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            for child in children {
                validate_schema_node(child, rel, file_path, root_document, problems);
            }
        }
    }

    for keyword in [
        "$defs",
        "definitions",
        "dependentSchemas",
        "patternProperties",
        "properties",
    ] {
        if let Some(children) = object.get(keyword).and_then(Value::as_object) {
            for child in children.values() {
                validate_schema_node(child, rel, file_path, root_document, problems);
            }
        }
    }
}

fn validate_keyword_shapes(object: &Map<String, Value>, rel: &str, problems: &mut Vec<Problem>) {
    for keyword in [
        "properties",
        "$defs",
        "definitions",
        "dependentSchemas",
        "patternProperties",
    ] {
        if let Some(value) = object.get(keyword) {
            if !value.is_object() {
                problems.push(Problem {
                    severity: Severity::Error,
                    path: rel.into(),
                    message: format!("'{keyword}' must be an object"),
                });
            }
        }
    }
    for keyword in [
        "required",
        "enum",
        "allOf",
        "anyOf",
        "oneOf",
        "prefixItems",
    ] {
        if let Some(value) = object.get(keyword) {
            if !value.is_array() {
                problems.push(Problem {
                    severity: Severity::Error,
                    path: rel.into(),
                    message: format!("'{keyword}' must be an array"),
                });
            }
        }
    }
    if let Some(value) = object.get("type") {
        let valid = value.is_string()
            || value
                .as_array()
                .map(|items| items.iter().all(Value::is_string))
                .unwrap_or(false);
        if !valid {
            problems.push(Problem {
                severity: Severity::Error,
                path: rel.into(),
                message: "'type' must be a string or array of strings".into(),
            });
        }
    }
}

fn validate_ref(
    object: &Map<String, Value>,
    rel: &str,
    file_path: &Path,
    root_document: &Value,
    problems: &mut Vec<Problem>,
) {
    let Some(reference) = object.get("$ref").and_then(Value::as_str) else {
        return;
    };
    if let Some(fragment) = reference.strip_prefix('#') {
        if resolve_json_pointer(root_document, fragment).is_none() {
            problems.push(Problem {
                severity: Severity::Error,
                path: rel.into(),
                message: format!("local $ref {reference:?} does not resolve"),
            });
        }
        return;
    }
    if reference.contains("://") {
        return;
    }
    let file_part = reference.split('#').next().unwrap_or(reference);
    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.join(file_part).exists() {
        problems.push(Problem {
            severity: Severity::Error,
            path: rel.into(),
            message: format!("$ref {reference:?} points at a missing file"),
        });
    }
}

fn resolve_json_pointer<'a>(root: &'a Value, fragment: &str) -> Option<&'a Value> {
    if fragment.is_empty() {
        return Some(root);
    }
    let pointer = fragment.strip_prefix('/').unwrap_or(fragment);
    let mut current = root;
    for raw in pointer.split('/') {
        let token = raw.replace("~1", "/").replace("~0", "~");
        current = match current {
            Value::Object(object) => object.get(&token)?,
            Value::Array(array) => array.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    return Some(current);
}

fn relative_display(root: &Path, path: &Path) -> String {
    return path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_names_that_match_schema_keywords_are_not_misclassified() {
        let document = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.test/schema.json",
            "type": "object",
            "properties": {
                "type": { "$ref": "#/$defs/scalar" },
                "required": { "type": "boolean" }
            },
            "$defs": {
                "scalar": { "type": "string" }
            }
        });
        let mut problems = Vec::new();
        validate_schema_node(
            &document,
            "schema.json",
            Path::new("schema.json"),
            &document,
            &mut problems,
        );
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn invalid_keyword_shape_is_still_rejected() {
        let document = serde_json::json!({
            "type": { "not": "valid" },
            "required": { "also": "invalid" }
        });
        let mut problems = Vec::new();
        validate_schema_node(
            &document,
            "schema.json",
            Path::new("schema.json"),
            &document,
            &mut problems,
        );
        assert_eq!(problems.len(), 2);
    }

    #[test]
    fn local_ref_must_resolve() {
        let document = serde_json::json!({ "$ref": "#/$defs/missing", "$defs": {} });
        let mut problems = Vec::new();
        validate_schema_node(
            &document,
            "schema.json",
            Path::new("schema.json"),
            &document,
            &mut problems,
        );
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("does not resolve"));
    }
}
