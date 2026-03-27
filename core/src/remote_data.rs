use std::path::Path;

use crate::enums::GenericTriState;
use crate::errors::{DomainError, Result};
use crate::roblox_api::RemoteDataBase;

/// remote data manager — fetches configuration from a remote URL with local fallback.
/// mirrors Ruststrap's `RemoteData.cs`.
pub struct RemoteDataManager {
    pub data: RemoteDataBase,
    pub loaded_state: GenericTriState,
    file_location: String,
}

impl RemoteDataManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            data: RemoteDataBase::default(),
            loaded_state: GenericTriState::Unknown,
            file_location: base_dir.join("Data.json").to_string_lossy().to_string(),
        }
    }

    /// load data from the remote URL, falling back to local cache.
    pub fn load_data(&mut self, remote_url: &str, force_local: bool) {
        if force_local {
            self.load_local();
            self.loaded_state = GenericTriState::Successful;
            return;
        }

        match self.fetch_remote(remote_url) {
            Ok(data) => {
                self.data = data;
                self.loaded_state = GenericTriState::Successful;
                // save to local cache
                self.save_local();
            }
            Err(_) => {
                self.load_local();
                self.loaded_state = GenericTriState::Failed;
            }
        }
    }

    fn fetch_remote(&self, url: &str) -> Result<RemoteDataBase> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Ruststrap/0.1")
            .build()
            .map_err(|e| DomainError::Network(format!("remote data client: {e}")))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| DomainError::Network(format!("remote data fetch: {e}")))?;

        let text = response
            .text()
            .map_err(|e| DomainError::Network(format!("remote data read: {e}")))?;

        let data: RemoteDataBase = serde_json::from_str(&text)
            .map_err(|e| DomainError::Serialization(format!("remote data parse: {e}")))?;

        Ok(data)
    }

    fn load_local(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.file_location) {
            if let Ok(data) = serde_json::from_str::<RemoteDataBase>(&text) {
                self.data = data;
            }
        }
    }

    fn save_local(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = std::fs::write(&self.file_location, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_data_default() {
        let mgr = RemoteDataManager::new(Path::new("/tmp"));
        assert_eq!(mgr.loaded_state, GenericTriState::Unknown);
        assert!(mgr.data.deeplink_url.is_empty());
    }
}
