/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutRequest {
    pub shortcut_path: PathBuf,
    pub target_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub icon_path: Option<PathBuf>,
    pub description: Option<String>,
}

pub trait ShortcutBackend {
    fn create_shortcut(&self, request: ShortcutRequest) -> Result<()>;

    fn remove_shortcut(&self, shortcut_path: PathBuf) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsShortcutBackend;

#[cfg(windows)]
impl ShortcutBackend for WindowsShortcutBackend {
    fn create_shortcut(&self, request: ShortcutRequest) -> Result<()> {
        if let Some(parent) = request.shortcut_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let shortcut = ps_single_quote(request.shortcut_path.to_string_lossy().as_ref());
        let target = ps_single_quote(request.target_path.to_string_lossy().as_ref());
        let args = ps_single_quote(&request.arguments.join(" "));
        let work_dir = request
            .working_directory
            .as_ref()
            .map(|path| ps_single_quote(path.to_string_lossy().as_ref()))
            .unwrap_or_else(|| "''".to_string());
        let icon = request
            .icon_path
            .as_ref()
            .map(|path| ps_single_quote(path.to_string_lossy().as_ref()))
            .unwrap_or_else(|| "''".to_string());
        let description = request
            .description
            .as_ref()
            .map(|value| ps_single_quote(value))
            .unwrap_or_else(|| "''".to_string());

        let script = format!(
            "$ws=New-Object -ComObject WScript.Shell; \
             $s=$ws.CreateShortcut({shortcut}); \
             $s.TargetPath={target}; \
             $s.Arguments={args}; \
             if ({work_dir} -ne '') {{$s.WorkingDirectory={work_dir};}}; \
             if ({icon} -ne '') {{$s.IconLocation={icon};}}; \
             if ({description} -ne '') {{$s.Description={description};}}; \
             $s.Save();"
        );

        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()?;
        if !status.success() {
            return Err(anyhow!("failed to create shortcut"));
        }
        Ok(())
    }

    fn remove_shortcut(&self, shortcut_path: PathBuf) -> Result<()> {
        if shortcut_path.exists() {
            std::fs::remove_file(shortcut_path)?;
        }
        Ok(())
    }
}

#[cfg(not(windows))]
impl ShortcutBackend for WindowsShortcutBackend {
    fn create_shortcut(&self, _request: ShortcutRequest) -> Result<()> {
        Ok(())
    }

    fn remove_shortcut(&self, _shortcut_path: PathBuf) -> Result<()> {
        Ok(())
    }
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
