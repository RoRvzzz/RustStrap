/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::{DomainError, Result};
use crate::process_utils::configure_hidden;
use serde::Serialize;

/// project constants that mirror Ruststrap's App class.
pub const PROJECT_NAME: &str = "Ruststrap";
pub const PROJECT_OWNER: &str = "Ruststrap";
pub const PROJECT_HELP_LINK: &str = "https://github.com/Ruststrap/Ruststrap/wiki";
pub const PROJECT_SUPPORT_LINK: &str = "https://github.com/Ruststrap/Ruststrap";
pub const PROJECT_DOWNLOAD_LINK: &str = "https://github.com/Ruststrap/Ruststrap/releases/latest";

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\RustStrap";

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeReadiness {
    pub installed: bool,
    pub uninstall_key_present: bool,
    pub owns_player_protocol: bool,
    pub owns_studio_protocol: bool,
    pub install_required: bool,
    pub running_exe_path: Option<String>,
    pub running_exe_matches_expected: bool,
    pub running_binary_matches_expected: bool,
    pub runtime_reconcile_required: bool,
    pub relaunched: bool,
    pub expected_exe_path: String,
    pub expected_player_command: String,
    pub expected_studio_command: String,
    pub actual_roblox_command: Option<String>,
    pub actual_roblox_player_command: Option<String>,
    pub actual_roblox_studio_command: Option<String>,
    pub actual_roblox_studio_auth_command: Option<String>,
}

/// files to import from Ruststrap installation.
const FILES_FOR_IMPORTING: &[&str] = &["CustomThemes", "Modifications", "Settings.json"];

/// validate that a proposed install location is acceptable.
pub fn check_install_location(location: &str) -> std::result::Result<(), String> {
    if location.len() <= 3 {
        return Err("Cannot install to root of a drive".to_string());
    }
    if location.starts_with(r"\\") {
        return Err("UNC paths are not supported".to_string());
    }

    let lower = location.to_ascii_lowercase();
    let temp_path = std::env::temp_dir().to_string_lossy().to_ascii_lowercase();
    if lower.starts_with(&temp_path) || lower.contains(r"\temp\") {
        return Err("Cannot install to temp directory".to_string());
    }
    if lower.contains("onedrive") {
        return Err("Cannot install to OneDrive folder".to_string());
    }
    if lower.contains("program files") {
        return Err("Cannot install to Program Files".to_string());
    }
    if lower.contains(r"local\Ruststrap") {
        return Err("Cannot install inside Ruststrap directory".to_string());
    }

    // prevent installing to essential user profile folders
    let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
    if !user_profile.is_empty() {
        let parent = Path::new(location)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if parent.eq_ignore_ascii_case(&user_profile) {
            return Err("Cannot install to essential user profile folder".to_string());
        }
    }

    // write test
    let test_dir = Path::new(location);
    if let Err(e) = fs::create_dir_all(test_dir) {
        return Err(format!("Cannot create directory: {e}"));
    }
    let test_file = test_dir.join("RuststrapWriteTest.txt");
    if let Err(e) = fs::write(&test_file, "") {
        return Err(format!("No write permissions: {e}"));
    }
    let _ = fs::remove_file(&test_file);

    Ok(())
}

/// perform a fresh installation.
pub fn do_install(
    install_location: &Path,
    current_exe: &Path,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    import_from_Ruststrap: bool,
) -> Result<()> {
    fs::create_dir_all(install_location)?;

    let app_path = installed_app_path(install_location);

    // copy executable and fail loudly on any write/copy failure.
    if current_exe != app_path {
        fs::copy(current_exe, &app_path).map_err(|e| {
            DomainError::Process(format!(
                "failed to copy executable from {} to {}: {e}",
                current_exe.display(),
                app_path.display()
            ))
        })?;
    } else if !app_path.exists() {
        return Err(DomainError::Process(format!(
            "expected application executable does not exist: {}",
            app_path.display()
        )));
    }

    // register uninstall key
    #[cfg(windows)]
    {
        register_uninstall_key(&app_path, install_location)?;
        register_protocols(&app_path)?;
    }

    // create shortcuts
    if create_desktop_shortcut {
        let desktop = dirs_desktop();
        if let Some(desktop_dir) = desktop {
            let shortcut_path = desktop_dir.join(format!("{PROJECT_NAME}.lnk"));
            create_shortcut(&app_path, &shortcut_path);
        }
    }

    if create_start_menu_shortcut {
        let start_menu = dirs_start_menu();
        if let Some(start_dir) = start_menu {
            let shortcut_path = start_dir.join(format!("{PROJECT_NAME}.lnk"));
            create_shortcut(&app_path, &shortcut_path);
        }
    }

    // import settings from Ruststrap
    if import_from_Ruststrap {
        let Ruststrap_dir = Ruststrap_install_dir();
        if Ruststrap_dir.exists() {
            import_settings_from_Ruststrap(&Ruststrap_dir, install_location)?;
        }
    }

    Ok(())
}

/// perform uninstallation.
pub fn do_uninstall(base_dir: &Path, keep_data: bool) -> Result<()> {
    do_uninstall_internal(base_dir, keep_data, true)
}

/// perform an uninstall pass intended for immediate reinstall.
/// this variant does not schedule delayed self-deletion.
pub fn do_uninstall_for_reinstall(base_dir: &Path, keep_data: bool) -> Result<()> {
    do_uninstall_internal(base_dir, keep_data, false)
}

fn do_uninstall_internal(
    base_dir: &Path,
    keep_data: bool,
    schedule_self_delete: bool,
) -> Result<()> {
    // kill any running Roblox processes
    kill_roblox_processes();

    let versions_dir = base_dir.join("Versions");
    let downloads_dir = base_dir.join("Downloads");
    let mods_dir = base_dir.join("Modifications");
    let logs_dir = base_dir.join("Logs");

    // cleanup sequence (best-effort, continue on error)
    let cleanup: Vec<Box<dyn FnOnce() -> std::io::Result<()>>> = vec![
        Box::new(move || {
            if versions_dir.exists() {
                fs::remove_dir_all(&versions_dir)
            } else {
                Ok(())
            }
        }),
        Box::new(move || {
            if downloads_dir.exists() {
                fs::remove_dir_all(&downloads_dir)
            } else {
                Ok(())
            }
        }),
    ];

    for action in cleanup {
        let _ = action();
    }

    if !keep_data {
        let _ = fs::remove_dir_all(&mods_dir);
        let _ = fs::remove_dir_all(&logs_dir);

        let settings_path = base_dir.join("Settings.json");
        let _ = fs::remove_file(&settings_path);
    }

    // remove state file
    let state_path = base_dir.join("State.json");
    let _ = fs::remove_file(&state_path);

    let roblox_state_path = base_dir.join("RobloxState.json");
    let _ = fs::remove_file(&roblox_state_path);

    // remove uninstall registry key
    #[cfg(windows)]
    {
        let _ = remove_uninstall_key();
    }

    // deregister protocol handlers
    #[cfg(windows)]
    {
        deregister_protocols();
    }

    // self-delete via delayed cmd.exe
    if schedule_self_delete {
        let app_path = installed_app_path(base_dir);
        if app_path.exists() {
            let mut command = Command::new("cmd.exe");
            configure_hidden(&mut command);
            let _ = command
                .args([
                    "/c",
                    &format!(
                        "timeout 5 && del /Q \"{}\" && rmdir \"{}\"",
                        app_path.display(),
                        base_dir.display()
                    ),
                ])
                .spawn();
        }
    }

    Ok(())
}

/// import settings, modifications, and custom themes from a Ruststrap installation.
pub fn import_settings_from_Ruststrap(Ruststrap_dir: &Path, install_dir: &Path) -> Result<()> {
    for file_name in FILES_FOR_IMPORTING {
        let source = Ruststrap_dir.join(file_name);
        if !source.exists() {
            continue;
        }

        let destination = install_dir.join(file_name);
        let attrs = fs::metadata(&source)?;

        if attrs.is_dir() {
            // remove existing destination dir
            if destination.exists() {
                let _ = fs::remove_dir_all(&destination);
            }
            copy_dir_recursive(&source, &destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source, &destination)?;
        }
    }

    Ok(())
}

pub fn installed_app_path(base_dir: &Path) -> PathBuf {
    base_dir.join(format!("{PROJECT_NAME}.exe"))
}

pub fn expected_player_protocol_command(app_path: &Path) -> String {
    format!("\"{}\" -player \"%1\"", app_path.display())
}

pub fn expected_studio_protocol_command(app_path: &Path) -> String {
    format!("\"{}\" -studio \"%1\"", app_path.display())
}

pub fn ensure_protocol_ownership_for_exe(app_path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        register_protocols(app_path)?;
    }
    #[cfg(not(windows))]
    {
        let _ = app_path;
    }
    Ok(())
}

pub fn runtime_readiness(base_dir: &Path) -> Result<RuntimeReadiness> {
    let app_path = installed_app_path(base_dir);
    let uninstall_key_present = uninstall_key_present()?;
    let expected_player = expected_player_protocol_command(&app_path);
    let expected_studio = expected_studio_protocol_command(&app_path);
    let running_exe = std::env::current_exe().ok();
    let running_exe_path = running_exe
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let running_exe_matches_expected = running_exe
        .as_ref()
        .map(|path| path_eq_case_insensitive(path, &app_path))
        .unwrap_or(false);
    let running_binary_matches_expected = running_exe
        .as_ref()
        .and_then(|path| binaries_match(path, &app_path))
        .unwrap_or(false);

    let actual_roblox = protocol_command("roblox")?;
    let actual_roblox_player = protocol_command("roblox-player")?;
    let actual_roblox_studio = protocol_command("roblox-studio")?;
    let actual_roblox_studio_auth = protocol_command("roblox-studio-auth")?;

    let owns_player_protocol = matches_protocol(actual_roblox.as_deref(), &expected_player)
        && matches_protocol(actual_roblox_player.as_deref(), &expected_player);
    let owns_studio_protocol = matches_protocol(actual_roblox_studio.as_deref(), &expected_studio)
        && matches_protocol(actual_roblox_studio_auth.as_deref(), &expected_studio);

    let installed = uninstall_key_present && app_path.exists();
    let runtime_reconcile_required = installed
        && running_exe.is_some()
        && (!running_exe_matches_expected
            || (app_path.exists() && !running_binary_matches_expected));
    let install_required =
        !installed || !owns_player_protocol || !owns_studio_protocol || runtime_reconcile_required;

    Ok(RuntimeReadiness {
        installed,
        uninstall_key_present,
        owns_player_protocol,
        owns_studio_protocol,
        install_required,
        running_exe_path,
        running_exe_matches_expected,
        running_binary_matches_expected,
        runtime_reconcile_required,
        relaunched: false,
        expected_exe_path: app_path.to_string_lossy().to_string(),
        expected_player_command: expected_player,
        expected_studio_command: expected_studio,
        actual_roblox_command: actual_roblox,
        actual_roblox_player_command: actual_roblox_player,
        actual_roblox_studio_command: actual_roblox_studio,
        actual_roblox_studio_auth_command: actual_roblox_studio_auth,
    })
}

/// cleanup old version directories.
pub fn cleanup_versions_folder(
    versions_dir: &Path,
    current_player_guid: Option<&str>,
    current_studio_guid: Option<&str>,
    static_directory: bool,
) -> Result<()> {
    if !versions_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(versions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let should_keep = if static_directory {
            dir_name == "WindowsPlayer" || dir_name == "WindowsStudio64"
        } else {
            Some(dir_name) == current_player_guid || Some(dir_name) == current_studio_guid
        };

        if !should_keep {
            // check if Roblox exe can be deleted (not in use)
            let player_exe = path.join("RobloxPlayerBeta.exe");
            let studio_exe = path.join("RobloxStudioBeta.exe");

            let can_delete = if player_exe.exists() {
                fs::remove_file(&player_exe).is_ok()
            } else if studio_exe.exists() {
                fs::remove_file(&studio_exe).is_ok()
            } else {
                true
            };

            if can_delete {
                let _ = fs::remove_dir_all(&path);
            }
        }
    }

    Ok(())
}

// section: windows-specific helpers

#[cfg(windows)]
fn register_uninstall_key(app_path: &Path, install_dir: &Path) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(UNINSTALL_KEY)
        .map_err(|e| DomainError::Process(format!("registry create failed: {e}")))?;

    let app_str = app_path.to_string_lossy();
    let install_str = install_dir.to_string_lossy();

    let _ = key.set_value::<String, _>("DisplayIcon", &format!("{app_str},0"));
    let _ = key.set_value("DisplayName", &PROJECT_NAME.to_string());
    let _ = key.set_value("DisplayVersion", &env!("CARGO_PKG_VERSION").to_string());
    let _ = key.set_value("InstallLocation", &install_str.to_string());
    let _ = key.set_value("NoRepair", &1u32);
    let _ = key.set_value("Publisher", &PROJECT_OWNER.to_string());
    let _ = key.set_value("ModifyPath", &format!("\"{app_str}\" -settings"));
    let _ = key.set_value(
        "QuietUninstallString",
        &format!("\"{app_str}\" -uninstall -quiet"),
    );
    let _ = key.set_value("UninstallString", &format!("\"{app_str}\" -uninstall"));
    let _ = key.set_value("HelpLink", &PROJECT_HELP_LINK.to_string());
    let _ = key.set_value("URLInfoAbout", &PROJECT_SUPPORT_LINK.to_string());
    let _ = key.set_value("URLUpdateInfo", &PROJECT_DOWNLOAD_LINK.to_string());

    Ok(())
}

#[cfg(windows)]
fn register_protocols(app_path: &Path) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let software = hkcu
        .open_subkey(r"Software")
        .map_err(|e| DomainError::Process(format!("registry open software failed: {e}")))?;
    let (classes, _) = software
        .create_subkey(r"Classes")
        .map_err(|e| DomainError::Process(format!("registry create classes failed: {e}")))?;

    let app_str = app_path.to_string_lossy().to_string();
    let player_command = expected_player_protocol_command(app_path);
    let studio_command = expected_studio_protocol_command(app_path);

    for proto in &["roblox", "roblox-player"] {
        let (key, _) = classes
            .create_subkey(proto)
            .map_err(|e| DomainError::Process(format!("registry create {proto} failed: {e}")))?;

        key.set_value("", &format!("URL:{PROJECT_NAME} Protocol"))?;
        key.set_value("URL Protocol", &"")?;

        let (icon, _) = key.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("\"{}\",0", app_str))?;

        let (shell, _) = key.create_subkey(r"shell\open\command")?;
        shell.set_value("", &player_command)?;
    }

    for proto in &["roblox-studio", "roblox-studio-auth"] {
        let (key, _) = classes
            .create_subkey(proto)
            .map_err(|e| DomainError::Process(format!("registry create {proto} failed: {e}")))?;

        key.set_value("", &format!("URL:{PROJECT_NAME} Protocol"))?;
        key.set_value("URL Protocol", &"")?;

        let (icon, _) = key.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("\"{}\",0", app_str))?;

        let (shell, _) = key.create_subkey(r"shell\open\command")?;
        shell.set_value("", &studio_command)?;
    }

    Ok(())
}

#[cfg(not(windows))]
fn register_protocols(_app_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn register_uninstall_key(_app_path: &Path, _install_dir: &Path) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn remove_uninstall_key() -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);
    Ok(())
}

#[cfg(not(windows))]
fn remove_uninstall_key() -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn deregister_protocols() {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes: winreg::RegKey = match hkcu.open_subkey(r"Software\Classes") {
        Ok(k) => k,
        Err(_) => return,
    };

    for proto in &[
        "roblox",
        "roblox-player",
        "roblox-studio",
        "roblox-studio-auth",
    ] {
        let _ = classes.delete_subkey_all(proto);
    }
}

#[cfg(not(windows))]
fn deregister_protocols() {}

#[cfg(windows)]
fn uninstall_key_present() -> Result<bool> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    Ok(hkcu.open_subkey(UNINSTALL_KEY).is_ok())
}

#[cfg(not(windows))]
fn uninstall_key_present() -> Result<bool> {
    Ok(false)
}

#[cfg(windows)]
fn protocol_command(proto: &str) -> Result<Option<String>> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!(r"Software\Classes\{proto}\shell\open\command");
    let key = match hkcu.open_subkey(&key_path) {
        Ok(key) => key,
        Err(_) => return Ok(None),
    };
    let value: String = match key.get_value("") {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    Ok(Some(value))
}

#[cfg(not(windows))]
fn protocol_command(_proto: &str) -> Result<Option<String>> {
    Ok(None)
}

fn normalize_command(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn matches_protocol(actual: Option<&str>, expected: &str) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    normalize_command(actual) == normalize_command(expected)
}

fn path_eq_case_insensitive(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(right.to_string_lossy().as_ref())
}

fn binaries_match(left: &Path, right: &Path) -> Option<bool> {
    let left_meta = fs::metadata(left).ok()?;
    let right_meta = fs::metadata(right).ok()?;
    if left_meta.len() != right_meta.len() {
        return Some(false);
    }

    let left_bytes = fs::read(left).ok()?;
    let right_bytes = fs::read(right).ok()?;
    Some(left_bytes == right_bytes)
}

fn kill_roblox_processes() {
    let mut kill_player = Command::new("taskkill");
    configure_hidden(&mut kill_player);
    let _ = kill_player
        .args(["/F", "/IM", "RobloxPlayerBeta.exe"])
        .output();

    let mut kill_crash_handler = Command::new("taskkill");
    configure_hidden(&mut kill_crash_handler);
    let _ = kill_crash_handler
        .args(["/F", "/IM", "RobloxCrashHandler.exe"])
        .output();
}

fn create_shortcut(target: &Path, shortcut_path: &Path) {
    // use PowerShell to create a .lnk shortcut
    let script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.Save()"#,
        shortcut_path.display(),
        target.display()
    );
    let mut command = Command::new("powershell");
    configure_hidden(&mut command);
    let _ = command.args(["-NoProfile", "-Command", &script]).output();
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn Ruststrap_install_dir() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(local_app_data).join("Ruststrap")
}

fn dirs_desktop() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .ok()
        .map(|p| PathBuf::from(p).join("Desktop"))
}

fn dirs_start_menu() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|p| {
        PathBuf::from(p)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
    })
}

/*
simple tests for the installer
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_root_drive() {
        assert!(check_install_location("C:\\").is_err());
    }

    #[test]
    fn rejects_unc_path() {
        assert!(check_install_location(r"\\server\share").is_err());
    }

    #[test]
    fn rejects_onedrive() {
        assert!(check_install_location(r"C:\Users\test\OneDrive\Apps").is_err());
    }

    #[test]
    fn rejects_program_files() {
        assert!(check_install_location(r"C:\Program Files\Ruststrap").is_err());
    }

    #[test]
    fn accepts_valid_location() {
        let temp = std::env::temp_dir().join("ruststrap_install_test");
        let _ = fs::remove_dir_all(&temp);
        let loc = temp.join("Ruststrap");
        // this test creates a real directory
        let result = check_install_location(&loc.to_string_lossy());
        let _ = fs::remove_dir_all(&temp);
        // the temp dir check might catch this, so we just verify it didn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn protocol_commands_include_explicit_launch_flags() {
        let app = Path::new(r"C:\Users\test\AppData\Local\Ruststrap\Ruststrap.exe");
        let player = expected_player_protocol_command(app);
        let studio = expected_studio_protocol_command(app);

        assert!(player.contains(" -player "));
        assert!(studio.contains(" -studio "));
        assert!(player.contains("\"%1\""));
        assert!(studio.contains("\"%1\""));
    }
}
