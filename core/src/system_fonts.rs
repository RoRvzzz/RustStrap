/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use serde::{Deserialize, Serialize};

use crate::errors::{DomainError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFontEntry {
    pub name: String,
    pub path: String,
}

#[cfg(windows)]
fn is_supported_font_file(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".ttf")
        || lower.ends_with(".otf")
        || lower.ends_with(".ttc")
        || lower.ends_with(".fon")
        || lower.ends_with(".fnt")
}

#[cfg(windows)]
fn normalize_font_name(value: &str) -> String {
    let mut normalized = value.trim().to_string();
    for suffix in [" (TrueType)", " (OpenType)"] {
        if normalized.ends_with(suffix) {
            normalized.truncate(normalized.len() - suffix.len());
            break;
        }
    }
    normalized.trim().to_string()
}

#[cfg(windows)]
fn collect_fonts_from_key(
    root: winreg::HKEY,
    key_path: &str,
    windows_fonts_dir: &std::path::Path,
    seen_paths: &mut std::collections::HashSet<String>,
    output: &mut Vec<SystemFontEntry>,
) -> Result<()> {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    let root = RegKey::predef(root);
    let key = match root.open_subkey_with_flags(key_path, KEY_READ) {
        Ok(key) => key,
        Err(_) => return Ok(()),
    };

    for value in key.enum_values() {
        let (name, _) = match value {
            Ok(pair) => pair,
            Err(_) => continue,
        };

        let file_value: String = match key.get_value(&name) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let file_value = file_value.trim_matches('\0').trim();
        if file_value.is_empty() || !is_supported_font_file(file_value) {
            continue;
        }

        let resolved_path = {
            let raw = std::path::PathBuf::from(file_value);
            if raw.is_absolute() {
                raw
            } else {
                windows_fonts_dir.join(raw)
            }
        };

        let canonical_key = resolved_path.to_string_lossy().to_ascii_lowercase();
        if !seen_paths.insert(canonical_key) {
            continue;
        }

        output.push(SystemFontEntry {
            name: normalize_font_name(&name),
            path: resolved_path.to_string_lossy().to_string(),
        });
    }

    Ok(())
}

pub fn list_system_fonts() -> Result<Vec<SystemFontEntry>> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

        let windows_dir = std::env::var("WINDIR")
            .map(std::path::PathBuf::from)
            .map_err(|err| DomainError::Process(format!("failed to resolve WINDIR: {err}")))?;
        let windows_fonts_dir = windows_dir.join("Fonts");

        let mut seen_paths = std::collections::HashSet::<String>::new();
        let mut fonts = Vec::<SystemFontEntry>::new();

        collect_fonts_from_key(
            HKEY_LOCAL_MACHINE, 
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
            &windows_fonts_dir,
            &mut seen_paths,
            &mut fonts,
        )?;

        collect_fonts_from_key(
            HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
            &windows_fonts_dir,
            &mut seen_paths,
            &mut fonts,
        )?;

        fonts.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.path.to_ascii_lowercase().cmp(&right.path.to_ascii_lowercase()))
        });
        return Ok(fonts);
    }

    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}
