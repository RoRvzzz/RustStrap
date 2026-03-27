/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/


use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::{DomainError, Result};

pub struct GlobalSettingsManager {
    pub file_location: PathBuf,
    pub loaded: bool,
    pub content: Option<String>,
    pub previous_read_only_state: bool,
}

/// preset paths for common settings (XPath-like, using `{UserSettings}` placeholder).
pub fn preset_paths() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert(
        "Rendering.FramerateCap",
        "{UserSettings}/int[@name='FramerateCap']",
    );
    m.insert(
        "Rendering.SavedQualityLevel",
        "{UserSettings}/token[@name='SavedQualityLevel']",
    );
    m.insert(
        "User.MouseSensitivity",
        "{UserSettings}/float[@name='MouseSensitivity']",
    );
    m.insert("User.VREnabled", "{UserSettings}/bool[@name='VREnabled']");
    m.insert(
        "UI.Transparency",
        "{UserSettings}/float[@name='PreferredTransparency']",
    );
    m.insert(
        "UI.ReducedMotion",
        "{UserSettings}/bool[@name='ReducedMotion']",
    );
    m.insert(
        "UI.FontSize",
        "{UserSettings}/token[@name='PreferredTextSize']",
    );
    m
}

const USER_SETTINGS_XPATH: &str = "//Item[@class='UserGameSettings']/Properties";

impl GlobalSettingsManager {
    pub fn new(roblox_dir: &Path) -> Self {
        Self {
            file_location: roblox_dir.join("GlobalBasicSettings_13.xml"),
            loaded: false,
            content: None,
            previous_read_only_state: false,
        }
    }

    /// load the XML document from disk.
    pub fn load(&mut self) -> Result<()> {
        if !self.file_location.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.file_location).map_err(|e| {
            DomainError::Process(format!("Failed to read GlobalBasicSettings: {e}"))
        })?;

        self.previous_read_only_state = self.get_read_only();
        self.content = Some(content);
        self.loaded = true;

        Ok(())
    }

    /// save the XML document to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(content) = &self.content {
            // temporarily remove read-only if needed
            self.set_read_only(false);

            fs::write(&self.file_location, content).map_err(|e| {
                DomainError::Process(format!("Failed to write GlobalBasicSettings: {e}"))
            })?;

            self.set_read_only(self.previous_read_only_state);
        }
        Ok(())
    }

    /// get a preset value by its key (e.g. "Rendering.FramerateCap").
    pub fn get_preset(&self, prefix: &str) -> Option<String> {
        let presets = preset_paths();
        let path_template = presets.get(prefix)?;
        self.get_value_by_template(path_template)
    }

    /// set a preset value by its key.
    pub fn set_preset(&mut self, prefix: &str, value: &str) {
        let presets = preset_paths();
        for (key, path_template) in &presets {
            if key.starts_with(prefix) {
                self.set_value_by_template(path_template, value);
            }
        }
    }

    /// simple XML value extraction using regex (avoids full XML parser dependency).
    fn get_value_by_template(&self, template: &str) -> Option<String> {
        let content = self.content.as_ref()?;
        let attr_name = extract_attr_name(template)?;
        let pattern = format!(r#"<[^>]*name=["']{attr_name}["'][^>]*>([^<]*)<"#);
        let re = regex::Regex::new(&pattern).ok()?;
        re.captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }


    fn set_value_by_template(&mut self, template: &str, value: &str) {
        let content = match self.content.as_ref() {
            Some(c) => c.clone(),
            None => return,
        };

        let attr_name = match extract_attr_name(template) {
            Some(n) => n,
            None => return,
        };

        let pattern = format!(r#"(<[^>]*name=["']{attr_name}["'][^>]*>)[^<]*(</)"#);
        if let Ok(re) = regex::Regex::new(&pattern) {
            let new_content = re
                .replace(&content, format!("${{1}}{value}${{2}}"))
                .to_string();
            self.content = Some(new_content);
        }
    }


    pub fn get_read_only(&self) -> bool {
        if !self.file_location.exists() {
            return false;
        }
        fs::metadata(&self.file_location)
            .map(|m| m.permissions().readonly())
            .unwrap_or(false)
    }

    /// set or clear the read-only attribute.
    pub fn set_read_only(&self, read_only: bool) {
        if !self.file_location.exists() {
            return;
        }
        if let Ok(meta) = fs::metadata(&self.file_location) {
            let mut perms = meta.permissions();
            perms.set_readonly(read_only);
            let _ = fs::set_permissions(&self.file_location, perms);
        }
    }
}

/// extract the attribute name from an XPath-like template.
/// e.g. `"{UserSettings}/int[@name='FramerateCap']"` → `"FramerateCap"`
fn extract_attr_name(template: &str) -> Option<String> {
    let re = regex::Regex::new(r"@name='([^']+)'").ok()?;
    re.captures(template)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_attr_name_works() {
        assert_eq!(
            extract_attr_name("{UserSettings}/int[@name='FramerateCap']"),
            Some("FramerateCap".to_string())
        );
        assert_eq!(
            extract_attr_name("{UserSettings}/bool[@name='VREnabled']"),
            Some("VREnabled".to_string())
        );
    }

    #[test]
    fn preset_paths_has_expected_keys() {
        let p = preset_paths();
        assert!(p.contains_key("Rendering.FramerateCap"));
        assert!(p.contains_key("UI.FontSize"));
        assert_eq!(p.len(), 7);
    }

    #[test]
    fn get_value_from_xml() {
        let mut mgr = GlobalSettingsManager::new(Path::new("/tmp"));
        mgr.content = Some(
            r#"<Item class="UserGameSettings"><Properties>
            <int name="FramerateCap">240</int>
            <bool name="VREnabled">false</bool>
            </Properties></Item>"#
                .to_string(),
        );
        mgr.loaded = true;

        assert_eq!(
            mgr.get_preset("Rendering.FramerateCap"),
            Some("240".to_string())
        );
        assert_eq!(mgr.get_preset("User.VREnabled"), Some("false".to_string()));
    }
}
