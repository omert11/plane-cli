use serde_json::{Map, Value};

/// Split a comma-separated string into trimmed, non-empty tokens.
pub fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}...")
    }
}

/// Insert a string field into a JSON body only when present (skip `None`).
pub fn insert_opt_str(body: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        body.insert(key.to_string(), Value::String(v));
    }
}

/// Insert a bool field only when present.
pub fn insert_opt_bool(body: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        body.insert(key.to_string(), Value::Bool(v));
    }
}

/// Insert a comma-separated string as a JSON array of strings only when present.
pub fn insert_opt_csv_array(body: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        let arr: Vec<Value> = split_csv(&v).into_iter().map(Value::String).collect();
        body.insert(key.to_string(), Value::Array(arr));
    }
}

/// A work-item human identifier like `PROJ-123` (project identifier + sequence).
pub struct WorkItemIdent {
    pub project_identifier: String,
    pub sequence: String,
}

/// Parse a `PROJ-123` style identifier (leading `#` optional). Returns `None`
/// when the shape doesn't match (no dash, empty side, non-numeric sequence).
pub fn parse_work_item_ident(s: &str) -> Option<WorkItemIdent> {
    let s = s.trim().trim_start_matches('#');
    let (proj, seq) = s.rsplit_once('-')?;
    if proj.is_empty() || seq.is_empty() || !seq.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(WorkItemIdent {
        project_identifier: proj.to_string(),
        sequence: seq.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_work_item_ident() {
        let id = parse_work_item_ident("PROJ-123").unwrap();
        assert_eq!(id.project_identifier, "PROJ");
        assert_eq!(id.sequence, "123");
        let id = parse_work_item_ident("#DSTK-7").unwrap();
        assert_eq!(id.project_identifier, "DSTK");
        assert_eq!(id.sequence, "7");
    }

    #[test]
    fn rejects_bad_ident() {
        assert!(parse_work_item_ident("PROJ").is_none());
        assert!(parse_work_item_ident("PROJ-").is_none());
        assert!(parse_work_item_ident("-123").is_none());
        assert!(parse_work_item_ident("PROJ-abc").is_none());
    }

    #[test]
    fn splits_csv() {
        assert_eq!(split_csv("a, b ,c"), vec!["a", "b", "c"]);
        assert!(split_csv("  , ").is_empty());
    }
}
