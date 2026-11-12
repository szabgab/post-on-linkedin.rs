use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn default_api_version() -> String {
    "202601".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub access_token: String,
    #[serde(default = "default_api_version")]
    pub api_version: String,
    #[serde(default)]
    pub person_id: Option<String>,
    #[serde(default)]
    pub organizations: BTreeMap<String, String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at '{}'", path.display()))?;
        let config: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in '{}'", path.display()))?;

        if config.access_token.trim().is_empty() {
            bail!("`access_token` in '{}' cannot be empty", path.display());
        }

        Ok(config)
    }

    /// Resolves the LinkedIn Author URN (`urn:li:person:...` or `urn:li:organization:...`).
    pub fn resolve_author_urn(&self, org_target: Option<&str>) -> Result<String> {
        match org_target {
            Some(target) => {
                let target = target.trim();
                if let Some(org_id) = self.organizations.get(target) {
                    Ok(normalize_urn(org_id, "organization"))
                } else if target.starts_with("urn:li:organization:")
                    || target.chars().all(|c| c.is_ascii_digit())
                {
                    Ok(normalize_urn(target, "organization"))
                } else {
                    let available: Vec<&str> =
                        self.organizations.keys().map(|k| k.as_str()).collect();
                    if available.is_empty() {
                        bail!(
                            "Organization alias '{}' not found in config.yaml (no organizations configured), and it is not a numeric ID.",
                            target
                        );
                    } else {
                        bail!(
                            "Organization alias '{}' not found in config.yaml. Available aliases: {}",
                            target,
                            available.join(", ")
                        );
                    }
                }
            }
            None => {
                let person_id = self
                    .person_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "YOUR_PERSON_ID")
                    .context(
                        "`person_id` is not set in config.yaml. Set `person_id` or run `whoami` to find your ID.",
                    )?;
                Ok(normalize_urn(person_id, "person"))
            }
        }
    }
}

pub fn normalize_urn(id_or_urn: &str, entity_type: &str) -> String {
    let trimmed = id_or_urn.trim();
    let prefix = format!("urn:li:{}:", entity_type);
    if trimmed.starts_with(&prefix) {
        trimmed.to_string()
    } else {
        format!("{}{}", prefix, trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        let yaml = r#"
access_token: "test_token"
api_version: "202601"
person_id: "abc123XYZ"
organizations:
  acme: "998877"
  full-urn-org: "urn:li:organization:112233"
"#;
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn resolves_person_urn() {
        let cfg = sample_config();
        assert_eq!(
            cfg.resolve_author_urn(None).unwrap(),
            "urn:li:person:abc123XYZ"
        );
    }

    #[test]
    fn resolves_org_alias_and_raw_id() {
        let cfg = sample_config();
        assert_eq!(
            cfg.resolve_author_urn(Some("acme")).unwrap(),
            "urn:li:organization:998877"
        );
        assert_eq!(
            cfg.resolve_author_urn(Some("full-urn-org")).unwrap(),
            "urn:li:organization:112233"
        );
        assert_eq!(
            cfg.resolve_author_urn(Some("555555")).unwrap(),
            "urn:li:organization:555555"
        );
    }

    #[test]
    fn errors_on_unknown_org_alias() {
        let cfg = sample_config();
        let err = cfg.resolve_author_urn(Some("unknown-org")).unwrap_err();
        assert!(err.to_string().contains("Available aliases: acme, full-urn-org"));
    }
}
