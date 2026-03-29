use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::errors::{DomainError, Result};

/// gitHub release information.
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Option<Vec<GitHubAsset>>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// result of checking for updates.
#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
}

/// update check
pub fn check_for_updates(current_version: &str, repo: &str) -> Result<UpdateCheckResult> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest"); /*   bad   */

    let client = reqwest::blocking::Client::builder()
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("http client build failed: {e}")))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("update check failed: {e}")))?;

    if !response.status().is_success() {
        return Ok(UpdateCheckResult {
            update_available: false,
            current_version: current_version.to_string(),
            latest_version: String::new(),
            download_url: None,
        });
    }

    let body = response
        .text()
        .map_err(|e| DomainError::Serialization(format!("release response read failed: {e}")))?;

    let release: GitHubRelease = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("release parse failed: {e}")))?;

    let comparison = compare_semver(current_version, &release.tag_name);
    let update_available = comparison == Ordering::Less;

    let download_url: Option<String> = release
        .assets
        .as_ref()
        .and_then(|a: &Vec<GitHubAsset>| a.first())
        .map(|asset| asset.browser_download_url.clone());

    Ok(UpdateCheckResult {
        update_available,
        current_version: current_version.to_string(),
        latest_version: release.tag_name,
        download_url,
    })
}

/// download and apply an update. Returns the path to the downloaded executable.
pub fn download_update(download_url: &str, temp_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(temp_dir)?;

    let file_name = download_url.rsplit('/').next().unwrap_or("update.exe");
    let download_path = temp_dir.join(file_name);

    if download_path.exists() {
        // already downloaded
        return Ok(download_path);
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("http client build failed: {e}")))?;

    let response = client
        .get(download_url)
        .send()
        .map_err(|e| DomainError::Network(format!("download failed: {e}")))?;

    let bytes = response
        .bytes()
        .map_err(|e| DomainError::Network(format!("download read failed: {e}")))?;

    fs::write(&download_path, &bytes)?;

    Ok(download_path)
}

/// launch the downloaded update executable with the -upgrade flag.
pub fn launch_update(
    update_exe: &Path,
    original_args: &[String],
    launch_mode: Option<&str>,
) -> Result<()> {
    let mut args = vec!["-upgrade".to_string()];
    for arg in original_args {
        args.push(arg.clone());
    }
    if let Some(mode) = launch_mode {
        let flag = format!("-{mode}");
        if !args.contains(&flag) {
            args.push(flag);
        }
    }

    Command::new(update_exe)
        .args(&args)
        .spawn()
        .map_err(|e| DomainError::Process(format!("failed to launch update: {e}")))?;

    Ok(())
}

/// compare two semantic version strings (e.g., "2.8.1" vs "2.9.0").
/// strips leading 'v' if present.
pub fn compare_semver(a: &str, b: &str) -> Ordering {
    let parse = |s: &str| -> Vec<u64> {
        let s = s.trim_start_matches('v');
        s.split('.').filter_map(|part| part.parse().ok()).collect()
    };

    let va = parse(a);
    let vb = parse(b);

    let max_len = va.len().max(vb.len());
    for i in 0..max_len {
        let a_part = va.get(i).copied().unwrap_or(0);
        let b_part = vb.get(i).copied().unwrap_or(0);
        match a_part.cmp(&b_part) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_semver_less() {
        assert_eq!(compare_semver("2.8.1", "2.9.0"), Ordering::Less);
        assert_eq!(compare_semver("1.0.0", "2.0.0"), Ordering::Less);
        assert_eq!(compare_semver("2.8.0", "2.8.1"), Ordering::Less);
    }

    #[test]
    fn compare_semver_equal() {
        assert_eq!(compare_semver("2.8.1", "2.8.1"), Ordering::Equal);
        assert_eq!(compare_semver("v2.8.1", "2.8.1"), Ordering::Equal);
    }

    #[test]
    fn compare_semver_greater() {
        assert_eq!(compare_semver("3.0.0", "2.9.9"), Ordering::Greater);
        assert_eq!(compare_semver("2.9.0", "2.8.1"), Ordering::Greater);
    }

    #[test]
    fn compare_semver_with_v_prefix() {
        assert_eq!(compare_semver("v1.2.3", "v1.2.4"), Ordering::Less);
    }
}
