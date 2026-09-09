//! Per-device model configuration: which providers the workspace routes
//! through the gateway, read from `model.json` beside the vault.
//!
//! Absent file → today's deterministic stand-ins, unchanged. A present but
//! malformed file is an error the founder sees, never a silent fallback:
//! silently downgrading "the model I configured" to a canned drafter would
//! be a lie about what produced a suggestion.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sovereign_model::{
    DeterministicProvider, Health as ModelHealth, ModelProvider, OllamaProvider,
};

use super::util::storage;
use super::WorkspaceError;

pub const MODEL_CONFIG_FILE: &str = "model.json";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub model: String,
}

fn default_enabled() -> bool {
    true
}

fn default_base_url() -> String {
    "http://127.0.0.1:11434".to_owned()
}

impl ModelConfig {
    pub fn load(root: &Path) -> Result<Self, WorkspaceError> {
        let path = root.join(MODEL_CONFIG_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&path).map_err(storage)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| WorkspaceError::Invalid(format!("{MODEL_CONFIG_FILE}: {error}")))
    }

    fn ollama_enabled(&self) -> Option<&OllamaConfig> {
        self.ollama.as_ref().filter(|ollama| ollama.enabled)
    }
}

/// The ordered provider list for this device: the configured Ollama daemon
/// first (loopback only; the product routes to it but does not confine it),
/// then the deterministic stand-ins so a down daemon fails over instead of
/// failing the founder's task.
pub fn providers_for(root: &Path) -> Result<Vec<Box<dyn ModelProvider>>, WorkspaceError> {
    let config = ModelConfig::load(root)?;
    let mut providers: Vec<Box<dyn ModelProvider>> = Vec::new();
    if let Some(ollama) = config.ollama_enabled() {
        let provider = OllamaProvider::new(
            format!("ollama:{}", ollama.model.trim()),
            &ollama.base_url,
            &ollama.model,
        )
        .map_err(|error| WorkspaceError::Invalid(format!("{MODEL_CONFIG_FILE}: {error}")))?;
        providers.push(Box::new(provider));
    }
    providers.push(Box::new(DeterministicProvider::local_echo(
        "local-drafter",
        ModelHealth::Healthy,
    )));
    providers.push(Box::new(DeterministicProvider::local_echo(
        "local-drafter-backup",
        ModelHealth::Healthy,
    )));
    Ok(providers)
}

/// What the founder sees on the settings page: each provider, where it runs,
/// its live health, and whether it is a real model or a canned stand-in.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub trust: String,
    pub health: String,
    pub real_model: bool,
}

pub fn provider_status(root: &Path) -> Result<Vec<ProviderStatus>, WorkspaceError> {
    Ok(providers_for(root)?
        .iter()
        .map(|provider| ProviderStatus {
            id: provider.id().to_owned(),
            trust: format!("{:?}", provider.trust()).to_lowercase(),
            health: format!("{:?}", provider.health()).to_lowercase(),
            real_model: provider.id().starts_with("ollama:"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_keeps_the_deterministic_stand_ins() {
        let dir = tempfile::tempdir().unwrap();
        let providers = providers_for(dir.path()).unwrap();
        let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();
        assert_eq!(ids, ["local-drafter", "local-drafter-backup"]);
    }

    #[test]
    fn an_enabled_ollama_entry_is_routed_first_and_reported_honestly() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MODEL_CONFIG_FILE),
            r#"{"version":1,"ollama":{"enabled":true,"base_url":"http://127.0.0.1:1","model":"qwen2.5:7b"}}"#,
        )
        .unwrap();
        let providers = providers_for(dir.path()).unwrap();
        assert_eq!(providers[0].id(), "ollama:qwen2.5:7b");
        assert_eq!(providers.len(), 3);
        // Nothing listens on port 1: status says so instead of pretending.
        let status = provider_status(dir.path()).unwrap();
        assert!(status[0].real_model);
        assert_eq!(status[0].trust, "local");
        assert_eq!(status[0].health, "down");
        assert!(!status[1].real_model);
    }

    #[test]
    fn disabled_entries_are_skipped_and_malformed_config_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MODEL_CONFIG_FILE),
            r#"{"ollama":{"enabled":false,"model":"x"}}"#,
        )
        .unwrap();
        assert_eq!(providers_for(dir.path()).unwrap().len(), 2);

        std::fs::write(dir.path().join(MODEL_CONFIG_FILE), "{not json").unwrap();
        let error = providers_for(dir.path()).unwrap_err();
        assert!(error.to_string().contains("model.json"));

        std::fs::write(
            dir.path().join(MODEL_CONFIG_FILE),
            r#"{"ollama":{"base_url":"http://10.0.0.9:11434","model":"x"}}"#,
        )
        .unwrap();
        assert!(
            providers_for(dir.path()).is_err(),
            "non-loopback must be refused"
        );
    }
}
