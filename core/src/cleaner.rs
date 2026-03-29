/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::errors::Result;

/// set max per run
const MAX_FILES_PER_DIR: usize = 200;

/// dirs
#[derive(Debug, Clone)]
pub struct CleanerConfig {
    pub ruststrap_logs: Option<PathBuf>,
    pub ruststrap_cache: Option<PathBuf>,
    pub roblox_logs: Option<PathBuf>,
    pub roblox_cache: Option<PathBuf>,
}

impl CleanerConfig {
    /// base builder
    pub fn from_base_dir(base_dir: &Path) -> Self {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        Self {
            ruststrap_logs: Some(base_dir.join("Logs")),
            ruststrap_cache: Some(base_dir.join("Downloads")),
            roblox_logs: if local_app_data.is_empty() {
                None
            } else {
                Some(PathBuf::from(&local_app_data).join("Roblox").join("logs"))
            },
            roblox_cache: if local_app_data.is_empty() {
                None
            } else {
                Some(PathBuf::from(&local_app_data).join("Roblox").join("cache"))
            },
        }
    }
}

/// age
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanerAge {
    OneDay,
    OneWeek,
    OneMonth,
    TwoMonths,
    Never,
}

impl CleanerAge {
    pub fn as_hours(&self) -> Option<u64> {
        match self {
            CleanerAge::OneDay => Some(24),
            CleanerAge::OneWeek => Some(24 * 7),
            CleanerAge::OneMonth => Some(24 * 30),
            CleanerAge::TwoMonths => Some(24 * 60),
            CleanerAge::Never => None,
        }
    }
}

/// run cleaner
pub fn run_cleaner(
    config: &CleanerConfig,
    age: CleanerAge,
    enabled_dirs: &[&str],
) -> Result<CleanerReport> {
    let threshold_hours = match age.as_hours() {
        Some(h) => h,
        None => return Ok(CleanerReport::default()),
    };

    let threshold = SystemTime::now() - Duration::from_secs(threshold_hours * 3600);
    let mut report = CleanerReport::default();

    let dirs: Vec<(&str, &Option<PathBuf>)> = vec![
        ("RuststrapLogs", &config.ruststrap_logs),
        ("RuststrapCache", &config.ruststrap_cache),
        ("RobloxLogs", &config.roblox_logs),
        ("RobloxCache", &config.roblox_cache),
    ];

    for (name, path_opt) in dirs {
        if !enabled_dirs.contains(&name) {
            continue;
        }

        let path = match path_opt {
            Some(p) if p.exists() => p,
            _ => continue,
        };

        let mut deleted_in_dir = 0;

        let files = match collect_files_recursive(path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for file in &files {
            if deleted_in_dir >= MAX_FILES_PER_DIR {
                break;
            }

            if !verify_file(file, threshold, path) {
                continue;
            }

            match fs::remove_file(file) {
                Ok(_) => {
                    deleted_in_dir += 1;
                    report.total_deleted += 1;
                }
                Err(_) => {
                    report.total_failed += 1;
                }
            }
        }
    }

    Ok(report)
}

#[derive(Debug, Clone, Default)]
pub struct CleanerReport {
    pub total_deleted: usize,
    pub total_failed: usize,
}

fn verify_file(file: &Path, threshold: SystemTime, base_path: &Path) -> bool {
    if !file.exists() {
        return false;
    }

    // check file age
    let created = match fs::metadata(file).and_then(|m| m.created()) {
        Ok(t) => t,
        Err(_) => return false,
    };

    if created > threshold {
        return false;
    }

    // safety: file must be within Roblox or Ruststrap directories
    let file_str = file.to_string_lossy();
    let base_str = base_path.to_string_lossy();

    if !file_str.contains("Roblox")
        && !file_str.contains("Ruststrap")
        && !file_str.starts_with(&*base_str)
    {
        return false;
    }

    // never touch Windows directory
    if file_str.contains("Windows") {
        return false;
    }

    true
}

fn collect_files_recursive(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Ok(sub_files) = collect_files_recursive(&path) {
                files.extend(sub_files);
            }
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaner_never_skips() {
        let config = CleanerConfig {
            ruststrap_logs: None,
            ruststrap_cache: None,
            roblox_logs: None,
            roblox_cache: None,
        };
        let report = run_cleaner(&config, CleanerAge::Never, &["RuststrapLogs"]).unwrap();
        assert_eq!(report.total_deleted, 0);
    }

    #[test]
    fn cleaner_age_hours() {
        assert_eq!(CleanerAge::OneDay.as_hours(), Some(24));
        assert_eq!(CleanerAge::OneWeek.as_hours(), Some(168));
        assert_eq!(CleanerAge::Never.as_hours(), None);
    }
}
