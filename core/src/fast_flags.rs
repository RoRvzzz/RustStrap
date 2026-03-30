/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::errors::{DomainError, Result};

/// preset flag definitions
pub static PRESET_FLAGS: &[(&str, &str)] = &[
    (
        "Rendering.ManualFullscreen",
        "FFlagHandleAltEnterFullscreenManually",
    ),
    ("Rendering.DisableScaling", "DFFlagDisableDPIScale"),
    ("Rendering.MSAA", "FIntDebugForceMSAASamples"),
    (
        "Rendering.FRMQualityOverride",
        "DFIntDebugFRMQualityLevelOverride",
    ),
    // rendering modes
    (
        "Rendering.Mode.DisableD3D11",
        "FFlagDebugGraphicsDisableDirect3D11",
    ),
    ("Rendering.Mode.D3D11", "FFlagDebugGraphicsPreferD3D11"),
    ("Rendering.Mode.Vulkan", "FFlagDebugGraphicsPreferVulkan"),
    ("Rendering.Mode.OpenGL", "FFlagDebugGraphicsPreferOpenGL"),
    // geometry / Mesh LOD
    (
        "Geometry.MeshLOD.Static",
        "DFIntCSGLevelOfDetailSwitchingDistanceStatic",
    ),
    (
        "Geometry.MeshLOD.L0",
        "DFIntCSGLevelOfDetailSwitchingDistance",
    ),
    (
        "Geometry.MeshLOD.L12",
        "DFIntCSGLevelOfDetailSwitchingDistanceL12",
    ),
    (
        "Geometry.MeshLOD.L23",
        "DFIntCSGLevelOfDetailSwitchingDistanceL23",
    ),
    (
        "Geometry.MeshLOD.L34",
        "DFIntCSGLevelOfDetailSwitchingDistanceL34",
    ),
    // texture quality
    (
        "Rendering.TextureQuality.OverrideEnabled",
        "DFFlagTextureQualityOverrideEnabled",
    ),
    (
        "Rendering.TextureQuality.Level",
        "DFIntTextureQualityOverride",
    ),
];

/// fastFlag manager that reads/writes `ClientSettings/ClientAppSettings.json`
pub struct FastFlagManager {
    file_path: PathBuf,
    flags: HashMap<String, Value>,
    original: HashMap<String, Value>,
}

impl FastFlagManager {
    pub fn new(modifications_dir: &Path) -> Self {
        let file_path = modifications_dir
            .join("ClientSettings")
            .join("ClientAppSettings.json");
        Self {
            file_path,
            flags: HashMap::new(),
            original: HashMap::new(),
        }
    }

    /// load flags from disk. If the file doesn't exist, starts with an empty set.
    pub fn load(&mut self) -> Result<()> {
        if self.file_path.exists() {
            let content = fs::read_to_string(&self.file_path)?;
            self.flags = serde_json::from_str(&content)
                .map_err(|e| DomainError::Serialization(format!("fast flags parse failed: {e}")))?;
        } else {
            self.flags = HashMap::new();
        }

        // force ManualFullscreen to False on load (matches C# behavior)
        if let Some(flag_name) = preset_flag_name("Rendering.ManualFullscreen") {
            let current = self.flags.get(flag_name).and_then(|v| v.as_str());
            if current != Some("False") {
                self.flags
                    .insert(flag_name.to_string(), Value::String("False".to_string()));
            }
        }

        self.original = self.flags.clone();
        Ok(())
    }

    /// save flags to disk. Creates the parent directory if needed.
    pub fn save(&mut self) -> Result<()> {
        // convert all values to strings before saving (matches C# behavior)
        let mut normalized = HashMap::new();
        for (key, value) in &self.flags {
            let str_val = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            normalized.insert(key.clone(), Value::String(str_val));
        }
        self.flags = normalized;

        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.flags)
            .map_err(|e| DomainError::Serialization(format!("fast flags serialize failed: {e}")))?;
        fs::write(&self.file_path, json)?;

        self.original = self.flags.clone();
        Ok(())
    }

    /// whether any flags have been modified since loading.
    pub fn changed(&self) -> bool {
        self.flags != self.original
    }

    /// set a flag value. Pass None to delete the flag.
    pub fn set_value(&mut self, key: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.flags
                .insert(key.to_string(), Value::String(v.to_string()));
        } else {
            self.flags.remove(key);
        }
    }

    /// get a flag value as a string, or None if it doesn't exist.
    pub fn get_value(&self, key: &str) -> Option<String> {
        self.flags.get(key).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        })
    }

    /// set all flags matching a preset prefix to the given value.
    pub fn set_preset(&mut self, prefix: &str, value: Option<&str>) {
        for (preset_key, flag_name) in PRESET_FLAGS {
            if preset_key.starts_with(prefix) {
                self.set_value(flag_name, value);
            }
        }
    }

    /// set a specific variant from a preset enum, clearing all others.
    pub fn set_preset_enum(&mut self, prefix: &str, target: &str, value: Option<&str>) {
        for (preset_key, flag_name) in PRESET_FLAGS {
            if preset_key.starts_with(prefix) {
                if preset_key.starts_with(&format!("{prefix}.{target}")) {
                    self.set_value(flag_name, value);
                } else {
                    self.set_value(flag_name, None);
                }
            }
        }
    }

    /// get the value of a named preset, or None if not set.
    pub fn get_preset(&self, name: &str) -> Option<String> {
        preset_flag_name(name).and_then(|flag_name| self.get_value(flag_name))
    }

    /// check if a flag name is a known preset flag.
    pub fn is_preset(&self, flag: &str) -> bool {
        let lower = flag.to_ascii_lowercase();
        PRESET_FLAGS
            .iter()
            .any(|(_, v)| v.to_ascii_lowercase() == lower)
    }

    /// return all current flags as a cloned map.
    pub fn all_flags(&self) -> HashMap<String, Value> {
        self.flags.clone()
    }

    /// replace all flags with the given map.
    pub fn replace_all(&mut self, flags: HashMap<String, Value>) {
        self.flags = flags;
    }
}

fn preset_flag_name(preset_key: &str) -> Option<&'static str> {
    PRESET_FLAGS
        .iter()
        .find(|(k, _)| *k == preset_key)
        .map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn set_and_get_value_round_trips() {
        let dir = std::env::temp_dir().join("ruststrap_ff_test_1");
        let _ = fs::remove_dir_all(&dir);
        let mut mgr = FastFlagManager::new(&dir);

        mgr.set_value("FFlagTest", Some("true"));
        assert_eq!(mgr.get_value("FFlagTest"), Some("true".to_string()));

        mgr.set_value("FFlagTest", None);
        assert_eq!(mgr.get_value("FFlagTest"), None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_persists_flags() {
        let dir = std::env::temp_dir().join("ruststrap_ff_test_2");
        let _ = fs::remove_dir_all(&dir);

        {
            let mut mgr = FastFlagManager::new(&dir);
            mgr.set_value("FFlagMyFlag", Some("42"));
            mgr.save().unwrap();
        }

        {
            let mut mgr = FastFlagManager::new(&dir);
            mgr.load().unwrap();
            assert_eq!(mgr.get_value("FFlagMyFlag"), Some("42".to_string()));
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preset_set_applies_to_all_matching_flags() {
        let dir = std::env::temp_dir().join("ruststrap_ff_test_3");
        let _ = fs::remove_dir_all(&dir);
        let mut mgr = FastFlagManager::new(&dir);

        mgr.set_preset("Rendering.Mode", Some("True"));

        assert_eq!(
            mgr.get_value("FFlagDebugGraphicsDisableDirect3D11"),
            Some("True".to_string())
        );
        assert_eq!(
            mgr.get_value("FFlagDebugGraphicsPreferVulkan"),
            Some("True".to_string())
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn changed_detects_modifications() {
        let dir = std::env::temp_dir().join("ruststrap_ff_test_4");
        let _ = fs::remove_dir_all(&dir);
        let mut mgr = FastFlagManager::new(&dir);
        mgr.load().unwrap();

        assert!(!mgr.changed());
        mgr.set_value("FFlagNew", Some("1"));
        assert!(mgr.changed());

        let _ = fs::remove_dir_all(&dir);
    }
}
