//! `.crossposter-state.json` : même format que la version TypeScript (ordre des clés conservé),
//! pour que le passage à Rust ne republie rien.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Devto,
    Linkedin,
    Medium,
}

impl Platform {
    pub const ALL: [Platform; 3] = [Platform::Devto, Platform::Linkedin, Platform::Medium];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Devto => "devto",
            Platform::Linkedin => "linkedin",
            Platform::Medium => "medium",
        }
    }

    pub fn parse(name: &str) -> Option<Platform> {
        Platform::ALL.into_iter().find(|platform| platform.name() == name)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRecord {
    pub posted_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl PostRecord {
    pub fn now() -> Self {
        PostRecord { posted_at: site::date::now_iso8601(), url: None, id: None }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct State(IndexMap<String, IndexMap<Platform, PostRecord>>);

impl State {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(State::default());
        }
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text).with_context(|| format!("state illisible : {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// Indentation de 2 espaces et saut de ligne final, comme `JSON.stringify(state, null, 2) + "\n"`.
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    pub fn is_done(&self, slug: &str, platform: Platform) -> bool {
        self.0.get(slug).is_some_and(|platforms| platforms.contains_key(&platform))
    }

    pub fn record(&mut self, slug: &str, platform: Platform, record: PostRecord) {
        self.0.entry(slug.to_string()).or_default().insert(platform, record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_state_round_trips_byte_for_byte() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.crossposter-state.json");
        let original = fs::read_to_string(&path).unwrap();
        let state: State = serde_json::from_str(&original).unwrap();
        assert_eq!(state.to_json().unwrap(), original);
    }
}
