use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

pub trait ShellBackend {
    fn open_url(&self, url: &str) -> Result<()>;

    fn open_path(&self, path: &Path) -> Result<()>;

    fn reveal_path(&self, path: &Path) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsShellBackend;

#[cfg(windows)]
impl ShellBackend for WindowsShellBackend {
    fn open_url(&self, url: &str) -> Result<()> {
        let status = Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()?;
        if !status.success() {
            return Err(anyhow!("failed to open url"));
        }
        Ok(())
    }

    fn open_path(&self, path: &Path) -> Result<()> {
        let status = Command::new("explorer").arg(path).status()?;
        if !status.success() {
            return Err(anyhow!("failed to open path"));
        }
        Ok(())
    }

    fn reveal_path(&self, path: &Path) -> Result<()> {
        let select_arg = format!("/select,{}", path.display());
        let status = Command::new("explorer").arg(select_arg).status()?;
        if !status.success() {
            return Err(anyhow!("failed to reveal path"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
impl ShellBackend for WindowsShellBackend {
    fn open_url(&self, _url: &str) -> Result<()> {
        Ok(())
    }

    fn open_path(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn reveal_path(&self, _path: &Path) -> Result<()> {
        Ok(())
    }
}
