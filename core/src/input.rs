use std::collections::HashMap;
use anyhow::{anyhow, Result};

#[derive(Debug, PartialEq)]
pub struct ParsedInput {
    pub name: String,
    pub metadata: HashMap<String, String>,
}

pub fn parse_args(args: &[String]) -> ParsedInput {
    let mut name_parts = Vec::new();
    let mut metadata = HashMap::new();

    for arg in args {
        if let Some((key, value)) = arg.split_once(':') {
            if !key.is_empty() {
                metadata.insert(key.to_string(), value.to_string());
                continue;
            }
        }
        name_parts.push(arg.as_str());
    }

    ParsedInput {
        name: name_parts.join(" "),
        metadata,
    }
}

pub fn expand_key(key: &str, candidates: &[&str]) -> Result<String> {
    // 1. Exact match
    if candidates.contains(&key) {
        return Ok(key.to_string());
    }

    // 2. Prefix match
    let matches: Vec<&str> = candidates
        .iter()
        .filter(|&&c| c.starts_with(key))
        .cloned()
        .collect();

    match matches.len() {
        1 => Ok(matches[0].to_string()),
        0 => Err(anyhow!("Unknown key: '{}'", key)),
        _ => Err(anyhow!("Ambiguous key: '{}' matches {:?}", key, matches)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let args = vec![
            "Buy".to_string(),
            "milk".to_string(),
            "due:tomorrow".to_string(),
            "project:Groceries".to_string(),
        ];
        let parsed = parse_args(&args);
        assert_eq!(parsed.name, "Buy milk");
        assert_eq!(parsed.metadata.get("due"), Some(&"tomorrow".to_string()));
        assert_eq!(parsed.metadata.get("project"), Some(&"Groceries".to_string()));
    }

    #[test]
    fn test_expand_key() {
        let candidates = vec!["due", "project", "priority"];
        
        assert_eq!(expand_key("d", &candidates).unwrap(), "due");
        assert_eq!(expand_key("du", &candidates).unwrap(), "due");
        assert_eq!(expand_key("due", &candidates).unwrap(), "due");
        
        assert_eq!(expand_key("pro", &candidates).unwrap(), "project");
        assert_eq!(expand_key("pri", &candidates).unwrap(), "priority");
        
        // Ambiguous
        assert!(expand_key("p", &candidates).is_err()); // matches project, priority
        assert!(expand_key("pr", &candidates).is_err()); // matches project, priority
        
        // Unknown
        assert!(expand_key("x", &candidates).is_err());
    }
}
