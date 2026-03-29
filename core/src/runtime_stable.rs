/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::cookies::{normalize_roblosecurity_cookie, persist_roblosecurity_cookie};
use crate::errors::{DomainError, Result};
use crate::launch_flags::LaunchMode;
use crate::orchestrator::BootstrapRuntime;
use crate::persistence::{
    parse_roblox_state_json, parse_settings_json, parse_state_json, to_pretty_json,
    ChannelChangeModeCompat, RobloxStateFileCompat, SettingsFileCompat, StateFileCompat,
};
use crate::process_utils::configure_hidden;

const DEFAULT_CHANNEL: &str = "production";
const BAD_CHANNEL_STATUSES: [u16; 3] = [401, 403, 404];

fn configured_roblosecurity_cookie(settings: &SettingsFileCompat) -> Option<String> {
    if !settings.allow_cookie_access {
        return None;
    }

    let normalized = normalize_roblosecurity_cookie(&settings.cookie_roblosecurity);
    if normalized.trim().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[derive(Debug, Clone)]
pub struct BootstrapRuntimeConfig {
    pub base_dir: PathBuf,
    pub settings_path: PathBuf,
    pub state_path: PathBuf,
    pub roblox_state_path: PathBuf,
    pub data_path: PathBuf,
    pub downloads_dir: PathBuf,
    pub versions_dir: PathBuf,
    pub modifications_dir: PathBuf,
}

impl BootstrapRuntimeConfig {
    pub fn from_base_dir(base_dir: PathBuf) -> Self {
        Self {
            settings_path: base_dir.join("Settings.json"),
            state_path: base_dir.join("State.json"),
            roblox_state_path: base_dir.join("RobloxState.json"),
            data_path: base_dir.join("Data.json"),
            downloads_dir: base_dir.join("Downloads"),
            versions_dir: base_dir.join("Versions"),
            modifications_dir: base_dir.join("Modifications"),
            base_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub signature: String,
    pub packed_size: u64,
    pub size: u64,
}

pub fn parse_package_manifest(data: &str) -> Result<Vec<PackageEntry>> {
    let mut lines = data.lines();
    let version = lines
        .next()
        .ok_or_else(|| DomainError::InvalidManifest("manifest is empty".to_string()))?;

    if version.trim() != "v0" {
        return Err(DomainError::InvalidManifest(format!(
            "unsupported manifest version `{}` (expected v0)",
            version.trim()
        )));
    }

    let mut packages = Vec::<PackageEntry>::new();
    loop {
        let Some(name_raw) = lines.next() else {
            break;
        };
        let Some(signature_raw) = lines.next() else {
            break;
        };
        let Some(packed_size_raw) = lines.next() else {
            break;
        };
        let Some(size_raw) = lines.next() else {
            break;
        };

        let name = name_raw.trim();
        if name.is_empty() {
            break;
        }
        // only include .zip packages for extraction
        if !name.ends_with(".zip") {
            continue;
        }

        let signature = signature_raw.trim();
        if signature.is_empty() {
            break;
        }

        let packed_size = packed_size_raw.trim().parse::<u64>().map_err(|err| {
            DomainError::InvalidManifest(format!(
                "invalid packed size `{}`: {err}",
                packed_size_raw
            ))
        })?;
        let size = size_raw.trim().parse::<u64>().map_err(|err| {
            DomainError::InvalidManifest(format!("invalid size `{}`: {err}", size_raw))
        })?;

        packages.push(PackageEntry {
            name: name.to_string(),
            signature: signature.to_string(),
            packed_size,
            size,
        });
    }

    Ok(packages)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersionInfo {
    pub version: String,
    pub version_guid: String,
    pub is_behind_default_channel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequestSpec {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub channel: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeploymentVersionOverride {
    VersionGuid(String),
    ClientVersionNumber(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserChannel {
    #[serde(rename = "channelName")]
    pub channel: String,
    #[serde(rename = "channelAssignmentType")]
    pub assignment_type: Option<i32>,
    #[serde(rename = "token")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEnrollment {
    pub channel: String,
    pub channel_token: Option<String>,
    pub launch_args: String,
}

fn is_default_channel(channel: &str) -> bool {
    channel.eq_ignore_ascii_case("production") || channel.eq_ignore_ascii_case("live")
}

pub fn compose_version_request(
    binary_type: &str,
    channel: Option<&str>,
    channel_token: Option<&str>,
) -> VersionRequestSpec {
    let resolved_channel = channel.unwrap_or("production");
    let path = if is_default_channel(resolved_channel) {
        format!("/v2/client-version/{binary_type}")
    } else {
        format!("/v2/client-version/{binary_type}/channel/{resolved_channel}")
    };

    let mut headers = HashMap::<String, String>::new();
    if let Some(token) = channel_token {
        if !token.trim().is_empty() {
            headers.insert("Roblox-Channel-Token".to_string(), token.to_string());
        }
    }

    VersionRequestSpec {
        url: format!("https://clientsettingscdn.roblox.com{path}"),
        headers,
        channel: resolved_channel.to_string(),
    }
}

fn compose_version_lookup_request(binary_type: &str, client_version: &str) -> VersionRequestSpec {
    let encoded_version = urlencoding::encode(client_version);
    VersionRequestSpec {
        url: format!(
            "https://clientsettingscdn.roblox.com/v2/client-version/{binary_type}?version={encoded_version}"
        ),
        headers: HashMap::new(),
        channel: DEFAULT_CHANNEL.to_string(),
    }
}

fn is_hex_string(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|char| char.is_ascii_hexdigit())
}

fn is_direct_version_number(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() >= 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|char| char.is_ascii_digit()))
}

fn parse_deployment_version_override(value: &str) -> Result<Option<DeploymentVersionOverride>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let lower = trimmed.to_ascii_lowercase();

    if let Some(hash) = lower.strip_prefix("version-") {
        if is_hex_string(hash) {
            return Ok(Some(DeploymentVersionOverride::VersionGuid(format!(
                "version-{hash}"
            ))));
        }

        return Err(DomainError::InvalidLaunchRequest(
            "invalid deployment override: `version-` values must contain only hex characters"
                .to_string(),
        ));
    }

    if is_direct_version_number(trimmed) {
        return Ok(Some(DeploymentVersionOverride::ClientVersionNumber(
            trimmed.to_string(),
        )));
    }

    if is_hex_string(&lower) {
        return Ok(Some(DeploymentVersionOverride::VersionGuid(format!(
            "version-{lower}"
        ))));
    }

    Err(DomainError::InvalidLaunchRequest(
        "invalid deployment override: expected a `version-<hash>`, bare hash, or a direct client version number".to_string(),
    ))
}

fn normalize_channel_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        DEFAULT_CHANNEL.to_string()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

fn extract_channel_from_launch_args(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let marker = "channel:";
    let index = lower.find(marker)?;
    let rest = &value[(index + marker.len())..];
    let channel = rest
        .chars()
        .take_while(|char| char.is_ascii_alphanumeric() || *char == '-' || *char == '_')
        .collect::<String>();
    if channel.is_empty() {
        None
    } else {
        Some(channel.to_ascii_lowercase())
    }
}

fn replace_uri_channel(value: &str, old_channel: &str, new_channel: &str) -> String {
    let old = format!("channel:{old_channel}");
    let lower_old = old.to_ascii_lowercase();
    let lower_value = value.to_ascii_lowercase();
    if let Some(index) = lower_value.find(&lower_old) {
        let mut out = String::with_capacity(value.len() + new_channel.len());
        out.push_str(&value[..index]);
        out.push_str(&format!("channel:{new_channel}"));
        out.push_str(&value[(index + old.len())..]);
        out
    } else {
        value.to_string()
    }
}

pub fn resolve_channel_enrollment(
    configured_channel: &str,
    launch_args: Option<&str>,
    channel_flag: Option<&str>,
    user_channel: Option<&UserChannel>,
    explicit_channel_token: Option<&str>,
    change_mode: ChannelChangeModeCompat,
) -> ChannelEnrollment {
    let launch_args = launch_args.unwrap_or_default();
    let mut channel = normalize_channel_name(configured_channel);
    let mut enrolled_channel = extract_channel_from_launch_args(launch_args)
        .unwrap_or_else(|| DEFAULT_CHANNEL.to_string());
    let mut launch_args_rewritten = launch_args.to_string();
    let mut channel_token = explicit_channel_token
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(user_channel) = user_channel {
        if let Some(token) = user_channel.token.clone() {
            if user_channel.assignment_type != Some(1) {
                channel_token = Some(token);
                if !user_channel.channel.trim().is_empty() {
                    if let Some(old_channel) = extract_channel_from_launch_args(launch_args) {
                        launch_args_rewritten =
                            replace_uri_channel(launch_args, &old_channel, &user_channel.channel);
                    }
                    enrolled_channel = normalize_channel_name(&user_channel.channel);
                }
            }
        }
    }

    if let Some(flag_channel) = channel_flag.filter(|value| !value.trim().is_empty()) {
        channel = normalize_channel_name(flag_channel);
    } else {
        match change_mode {
            ChannelChangeModeCompat::Automatic => {
                channel = normalize_channel_name(&enrolled_channel)
            }
            ChannelChangeModeCompat::Prompt | ChannelChangeModeCompat::Ignore => {}
        }
    }

    ChannelEnrollment {
        channel,
        channel_token,
        launch_args: launch_args_rewritten,
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct PackageMapsOverride {
    #[serde(rename = "common")]
    common: HashMap<String, String>,
    #[serde(rename = "player")]
    player: HashMap<String, String>,
    #[serde(rename = "studio")]
    studio: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct DataOverride {
    #[serde(rename = "packageMaps", alias = "PackageMaps")]
    package_maps: PackageMapsOverride,
}

fn default_common_package_map() -> HashMap<String, String> {
    HashMap::from_iter([
        ("Libraries.zip".to_string(), "".to_string()),
        ("redist.zip".to_string(), "".to_string()),
        ("shaders.zip".to_string(), "shaders\\".to_string()),
        ("ssl.zip".to_string(), "ssl\\".to_string()),
        ("WebView2.zip".to_string(), "".to_string()),
        (
            "WebView2RuntimeInstaller.zip".to_string(),
            "WebView2RuntimeInstaller\\".to_string(),
        ),
        (
            "content-avatar.zip".to_string(),
            "content\\avatar\\".to_string(),
        ),
        (
            "content-configs.zip".to_string(),
            "content\\configs\\".to_string(),
        ),
        (
            "content-fonts.zip".to_string(),
            "content\\fonts\\".to_string(),
        ),
        ("content-sky.zip".to_string(), "content\\sky\\".to_string()),
        (
            "content-sounds.zip".to_string(),
            "content\\sounds\\".to_string(),
        ),
        (
            "content-textures2.zip".to_string(),
            "content\\textures\\".to_string(),
        ),
        (
            "content-models.zip".to_string(),
            "content\\models\\".to_string(),
        ),
        (
            "content-textures3.zip".to_string(),
            "PlatformContent\\pc\\textures\\".to_string(),
        ),
        (
            "content-terrain.zip".to_string(),
            "PlatformContent\\pc\\terrain\\".to_string(),
        ),
        (
            "content-platform-fonts.zip".to_string(),
            "PlatformContent\\pc\\fonts\\".to_string(),
        ),
        (
            "content-platform-dictionaries.zip".to_string(),
            "PlatformContent\\pc\\shared_compression_dictionaries\\".to_string(),
        ),
        (
            "extracontent-luapackages.zip".to_string(),
            "ExtraContent\\LuaPackages\\".to_string(),
        ),
        (
            "extracontent-translations.zip".to_string(),
            "ExtraContent\\translations\\".to_string(),
        ),
        (
            "extracontent-models.zip".to_string(),
            "ExtraContent\\models\\".to_string(),
        ),
        (
            "extracontent-textures.zip".to_string(),
            "ExtraContent\\textures\\".to_string(),
        ),
        (
            "extracontent-places.zip".to_string(),
            "ExtraContent\\places\\".to_string(),
        ),
    ])
}

fn default_player_package_map() -> HashMap<String, String> {
    HashMap::from_iter([("RobloxApp.zip".to_string(), "".to_string())])
}

fn default_studio_package_map() -> HashMap<String, String> {
    HashMap::from_iter([
        ("RobloxStudio.zip".to_string(), "".to_string()),
        ("LibrariesQt5.zip".to_string(), "".to_string()),
        (
            "content-studio_svg_textures.zip".to_string(),
            "content\\studio_svg_textures\\".to_string(),
        ),
        (
            "content-qt_translations.zip".to_string(),
            "content\\qt_translations\\".to_string(),
        ),
        (
            "content-api-docs.zip".to_string(),
            "content\\api_docs\\".to_string(),
        ),
        (
            "extracontent-scripts.zip".to_string(),
            "ExtraContent\\scripts\\".to_string(),
        ),
        (
            "studiocontent-models.zip".to_string(),
            "StudioContent\\models\\".to_string(),
        ),
        (
            "studiocontent-textures.zip".to_string(),
            "StudioContent\\textures\\".to_string(),
        ),
        (
            "BuiltInPlugins.zip".to_string(),
            "BuiltInPlugins\\".to_string(),
        ),
        (
            "BuiltInStandalonePlugins.zip".to_string(),
            "BuiltInStandalonePlugins\\".to_string(),
        ),
        (
            "ApplicationConfig.zip".to_string(),
            "ApplicationConfig\\".to_string(),
        ),
        ("Plugins.zip".to_string(), "Plugins\\".to_string()),
        ("Qml.zip".to_string(), "Qml\\".to_string()),
        ("StudioFonts.zip".to_string(), "StudioFonts\\".to_string()),
        ("RibbonConfig.zip".to_string(), "RibbonConfig\\".to_string()),
    ])
}

fn merge_map(
    mut base: HashMap<String, String>,
    override_values: HashMap<String, String>,
) -> HashMap<String, String> {
    for (key, value) in override_values {
        base.insert(key, value);
    }
    base
}

pub fn build_package_map(
    mode: LaunchMode,
    local_data_json: Option<&str>,
) -> Result<HashMap<String, String>> {
    let mut common = default_common_package_map();
    let mut local = match mode {
        LaunchMode::Studio | LaunchMode::StudioAuth => default_studio_package_map(),
        _ => default_player_package_map(),
    };

    if let Some(raw) = local_data_json {
        let raw = raw.trim();
        if !raw.is_empty() {
            let override_data: DataOverride = serde_json::from_str(raw).map_err(|err| {
                DomainError::Serialization(format!("failed to parse Data.json packageMaps: {err}"))
            })?;
            common = merge_map(common, override_data.package_maps.common);
            local = match mode {
                LaunchMode::Studio | LaunchMode::StudioAuth => {
                    merge_map(local, override_data.package_maps.studio)
                }
                _ => merge_map(local, override_data.package_maps.player),
            };
        }
    }

    Ok(merge_map(common, local))
}

fn normalized_path(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

pub fn resolve_restore_package(
    mod_manifest_path: &str,
    package_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    let normalized_mod_path = normalized_path(mod_manifest_path);
    let mut matches = package_map
        .iter()
        .filter_map(|(package, directory)| {
            if directory.is_empty() {
                return None;
            }
            let normalized_directory = normalized_path(directory);
            if normalized_mod_path.starts_with(&normalized_directory) {
                Some((package.clone(), directory.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<(String, String)>>();

    matches.sort_by_key(|(_, directory)| std::cmp::Reverse(directory.len()));
    let (package_name, directory) = matches.into_iter().next()?;

    let relative = mod_manifest_path
        .replace('/', "\\")
        .strip_prefix(&directory)
        .unwrap_or(mod_manifest_path)
        .trim_start_matches('\\')
        .to_string();

    Some((package_name, relative))
}

fn process_launch_arguments() -> Vec<String> {
    if let Ok(args) = std::env::var("Ruststrap_LAUNCH_ARGS") {
        if !args.trim().is_empty() {
            return vec![args];
        }
    }
    std::env::args().skip(1).collect::<Vec<_>>()
}

fn extract_channel_flag(args: &[String]) -> Option<String> {
    let mut index = 0usize;
    while index < args.len() {
        if args[index].eq_ignore_ascii_case("-channel") && index + 1 < args.len() {
            return Some(args[index + 1].clone());
        }
        index += 1;
    }
    None
}

fn binary_type_for_mode(mode: LaunchMode) -> &'static str {
    match mode {
        LaunchMode::Studio | LaunchMode::StudioAuth => "WindowsStudio64",
        _ => "WindowsPlayer",
    }
}

fn priority_flag_from_selection(selection: i32) -> u32 {
    match selection {
        0 => 0x00000040, // IDLE
        1 => 0x00004000, // BELOW_NORMAL
        2 => 0,          // NORMAL
        3 => 0x00008000, // ABOVE_NORMAL
        4 => 0x00000080, // HIGH
        5 => 0x00000100, // REALTIME
        _ => 0,          // NORMAL
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct CustomIntegrationEntry {
    #[serde(rename = "Name", alias = "name")]
    name: String,
    #[serde(rename = "Location", alias = "location")]
    location: String,
    #[serde(rename = "LaunchArgs", alias = "launch_args")]
    launch_args: String,
    #[serde(rename = "Delay", alias = "delay")]
    delay: i64,
    #[serde(rename = "AutoClose", alias = "auto_close")]
    auto_close: bool,
    #[serde(rename = "PreLaunch", alias = "pre_launch")]
    pre_launch: bool,
}

fn parse_custom_integrations(raw: &[serde_json::Value]) -> Vec<CustomIntegrationEntry> {
    raw.iter()
        .filter_map(|value| serde_json::from_value::<CustomIntegrationEntry>(value.clone()).ok())
        .filter(|entry| !entry.location.trim().is_empty())
        .collect::<Vec<_>>()
}

fn split_launch_args(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in raw.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    out.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn launch_custom_integration_process(entry: &CustomIntegrationEntry) -> Option<u32> {
    if entry.delay > 0 {
        std::thread::sleep(std::time::Duration::from_millis(entry.delay as u64));
    }

    let mut command = Command::new(&entry.location);
    configure_hidden(&mut command);
    let args = split_launch_args(&entry.launch_args);
    if !args.is_empty() {
        command.args(args);
    }

    command.spawn().ok().map(|child| child.id())
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = left
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();
    let right_parts = right
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();
    let max = left_parts.len().max(right_parts.len());
    for index in 0..max {
        let l = *left_parts.get(index).unwrap_or(&0);
        let r = *right_parts.get(index).unwrap_or(&0);
        match l.cmp(&r) {
            Ordering::Equal => {}
            non_equal => return non_equal,
        }
    }
    Ordering::Equal
}

fn normalize_relative_path(value: &str) -> String {
    value
        .replace('/', "\\")
        .trim_start_matches('\\')
        .to_string()
}

fn http_get(url: &str, headers: &HashMap<String, String>) -> Result<(u16, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("http client build failed: {e}")))?;

    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key.as_str(), value.as_str());
    }

    let response = request
        .send()
        .map_err(|e| DomainError::Network(format!("http request failed: {e}")))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("http response read failed: {e}")))?;

    Ok((status, body))
}

fn http_download(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("http client build failed: {e}")))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| DomainError::Network(format!("download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(DomainError::Network(format!(
            "download failed: HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .map_err(|e| DomainError::Network(format!("download read failed: {e}")))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dest, &bytes)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ClientVersionWire {
    #[serde(rename = "version")]
    version: String,
    #[serde(rename = "clientVersionUpload")]
    version_guid: String,
}

fn fetch_client_version(spec: &VersionRequestSpec) -> Result<ClientVersionInfo> {
    let (status, body) = http_get(&spec.url, &spec.headers)?;
    if !(200..300).contains(&status) {
        if !is_default_channel(&spec.channel) && BAD_CHANNEL_STATUSES.contains(&status) {
            return Err(DomainError::InvalidChannelStatus(status));
        }
        return Err(DomainError::Network(format!(
            "failed requesting {}: http {}",
            spec.url, status
        )));
    }

    let wire: ClientVersionWire = serde_json::from_str(&body).map_err(|err| {
        DomainError::Serialization(format!("failed to parse client version response: {err}"))
    })?;

    Ok(ClientVersionInfo {
        version: wire.version,
        version_guid: wire.version_guid,
        is_behind_default_channel: false,
    })
}

fn fetch_client_version_with_fallback(spec: &VersionRequestSpec) -> Result<ClientVersionInfo> {
    match fetch_client_version(spec) {
        Ok(value) => Ok(value),
        Err(DomainError::InvalidChannelStatus(status)) => {
            Err(DomainError::InvalidChannelStatus(status))
        }
        Err(_) => {
            let fallback = VersionRequestSpec {
                url: spec
                    .url
                    .replace("clientsettingscdn.roblox.com", "clientsettings.roblox.com"),
                headers: spec.headers.clone(),
                channel: spec.channel.clone(),
            };
            fetch_client_version(&fallback)
        }
    }
}

fn extract_archive_native(archive: &Path, destination: &Path) -> Result<()> {
    let file = fs::File::open(archive).map_err(|e| {
        DomainError::Zip(format!("failed to open archive {}: {e}", archive.display()))
    })?;
    let mut file = file;
    // bro im so done
    // do i even know rust atp
    use std::io::{Read, Seek};
    let mut magic = [0u8; 4];
    let _ = file.read_exact(&mut magic);
    if &magic != b"PK\x03\x04" {
        return Err(DomainError::Zip(format!(
            "invalid zip header in {}: expected PK magic, got {:02X?}",
            archive.display(),
            magic
        )));
    }
    let _ = file.seek(std::io::SeekFrom::Start(0));

    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| DomainError::Zip(format!("failed to read zip {}: {e}", archive.display())))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| DomainError::Zip(format!("failed to read zip entry {i}: {e}")))?;
        let Some(path) = entry.enclosed_name().map(|p| destination.join(p)) else {
            continue;
        };
        if entry.is_dir() {
            fs::create_dir_all(&path).ok();
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut out = fs::File::create(&path).map_err(|e| {
                DomainError::Zip(format!("failed to create {}: {e}", path.display()))
            })?;
            std::io::copy(&mut entry, &mut out).map_err(|e| {
                DomainError::Zip(format!("failed to extract to {}: {e}", path.display()))
            })?;
        }
    }
    Ok(())
}

fn extract_selected_files_native(
    archive: &Path,
    destination: &Path,
    files: &[String],
) -> Result<()> {
    use std::collections::HashSet;
    let target: HashSet<String> = files.iter().map(|f| f.replace('/', "\\")).collect();
    let file = fs::File::open(archive).map_err(|e| {
        DomainError::Zip(format!("failed to open archive {}: {e}", archive.display()))
    })?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| DomainError::Zip(format!("failed to read zip {}: {e}", archive.display())))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| DomainError::Zip(format!("failed to read zip entry {i}: {e}")))?;
        let entry_path = entry.name().replace('/', "\\");
        if !target.contains(&entry_path) {
            continue;
        }
        let Some(path) = entry.enclosed_name().map(|p| destination.join(p)) else {
            continue;
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut out = fs::File::create(&path)
            .map_err(|e| DomainError::Zip(format!("failed to create {}: {e}", path.display())))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| {
            DomainError::Zip(format!("failed to extract to {}: {e}", path.display()))
        })?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct FilesystemBootstrapRuntime {
    pub config: BootstrapRuntimeConfig,
}

impl FilesystemBootstrapRuntime {
    pub fn new(config: BootstrapRuntimeConfig) -> Result<Self> {
        let runtime = Self { config };
        runtime.install_layout()?;
        Ok(runtime)
    }

    pub fn install_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.config.base_dir)?;
        fs::create_dir_all(&self.config.downloads_dir)?;
        fs::create_dir_all(&self.config.versions_dir)?;
        fs::create_dir_all(&self.config.modifications_dir)?;
        Ok(())
    }

    pub fn uninstall_layout(&self) -> Result<()> {
        if self.config.base_dir.exists() {
            fs::remove_dir_all(&self.config.base_dir)?;
        }
        Ok(())
    }

    pub fn load_settings(&self) -> Result<SettingsFileCompat> {
        if !self.config.settings_path.exists() {
            let settings = SettingsFileCompat::default();
            self.save_settings(&settings)?;
            return Ok(settings);
        }

        let raw = fs::read_to_string(&self.config.settings_path)?;
        parse_settings_json(&raw)
    }

    pub fn save_settings(&self, settings: &SettingsFileCompat) -> Result<()> {
        let json = to_pretty_json(settings)?;
        self.write_file(&self.config.settings_path, &json)
    }

    pub fn load_state(&self) -> Result<StateFileCompat> {
        if !self.config.state_path.exists() {
            let state = StateFileCompat::default();
            self.save_state(&state)?;
            return Ok(state);
        }
        let raw = fs::read_to_string(&self.config.state_path)?;
        parse_state_json(&raw)
    }

    pub fn save_state(&self, state: &StateFileCompat) -> Result<()> {
        let json = to_pretty_json(state)?;
        self.write_file(&self.config.state_path, &json)
    }

    pub fn load_roblox_state(&self) -> Result<RobloxStateFileCompat> {
        if !self.config.roblox_state_path.exists() {
            let state = RobloxStateFileCompat::default();
            self.save_roblox_state(&state)?;
            return Ok(state);
        }
        let raw = fs::read_to_string(&self.config.roblox_state_path)?;
        parse_roblox_state_json(&raw)
    }

    pub fn save_roblox_state(&self, state: &RobloxStateFileCompat) -> Result<()> {
        let json = to_pretty_json(state)?;
        self.write_file(&self.config.roblox_state_path, &json)
    }

    pub fn set_watcher_running(&self, running: bool) -> Result<()> {
        let mut state = self.load_state()?;
        state.watcher_running = running;
        self.save_state(&state)
    }

    pub fn current_version_for_mode(&self, mode: LaunchMode) -> Result<Option<String>> {
        let state = self.load_roblox_state()?;
        let value = match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => state.studio.version_guid,
            _ => state.player.version_guid,
        };
        if value.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn check_updates_for_mode(&self, mode: LaunchMode) -> Result<ClientVersionInfo> {
        let settings = self.load_settings()?;
        let args = process_launch_arguments();
        let launch_args = args.join(" ");
        let channel_flag = extract_channel_flag(&args);
        let configured_channel = normalize_channel_name(&settings.channel);
        let binary_type = binary_type_for_mode(mode);

        if let Some(override_mode) = parse_deployment_version_override(&settings.channel_hash)? {
            return match override_mode {
                DeploymentVersionOverride::VersionGuid(version_guid) => Ok(ClientVersionInfo {
                    version: version_guid.clone(),
                    version_guid,
                    is_behind_default_channel: false,
                }),
                DeploymentVersionOverride::ClientVersionNumber(client_version) => {
                    let request = compose_version_lookup_request(binary_type, &client_version);
                    fetch_client_version_with_fallback(&request)
                }
            };
        }

        let user_channel = self.try_fetch_user_channel(binary_type, &settings)?;

        let enrollment = resolve_channel_enrollment(
            &configured_channel,
            Some(&launch_args),
            channel_flag.as_deref(),
            user_channel.as_ref(),
            std::env::var("Ruststrap_CHANNEL_TOKEN").ok().as_deref(),
            settings.channel_change_mode,
        );

        let request = compose_version_request(
            binary_type,
            Some(&enrollment.channel),
            enrollment.channel_token.as_deref(),
        );
        let mut client_version = fetch_client_version_with_fallback(&request)?;

        if settings.channel_change_mode == ChannelChangeModeCompat::Prompt
            && !is_default_channel(&request.channel)
        {
            let default_request = compose_version_request(binary_type, Some(DEFAULT_CHANNEL), None);
            if let Ok(default_version) = fetch_client_version_with_fallback(&default_request) {
                client_version.is_behind_default_channel =
                    compare_versions(&client_version.version, &default_version.version)
                        == Ordering::Less;
            }
        }

        Ok(client_version)
    }

    pub fn package_map_for_mode(&self, mode: LaunchMode) -> Result<HashMap<String, String>> {
        let data_json = if self.config.data_path.exists() {
            Some(fs::read_to_string(&self.config.data_path)?)
        } else {
            None
        };
        build_package_map(mode, data_json.as_deref())
    }

    fn infer_mode_for_version(&self, version_guid: &str) -> Result<LaunchMode> {
        let state = self.load_roblox_state()?;
        if state.studio.version_guid.eq_ignore_ascii_case(version_guid) {
            Ok(LaunchMode::Studio)
        } else {
            Ok(LaunchMode::Player)
        }
    }

    fn try_fetch_user_channel(
        &self,
        binary_type: &str,
        settings: &SettingsFileCompat,
    ) -> Result<Option<UserChannel>> {
        if !settings.allow_cookie_access {
            return Ok(None);
        }

        if let Ok(raw) = std::env::var("Ruststrap_USER_CHANNEL_JSON") {
            if let Ok(parsed) = serde_json::from_str::<UserChannel>(&raw) {
                return Ok(Some(parsed));
            }
        }

        let env_cookie = std::env::var("Ruststrap_ROBLOSECURITY")
            .ok()
            .and_then(|value| {
                let normalized = normalize_roblosecurity_cookie(&value);
                if normalized.trim().is_empty() {
                    None
                } else {
                    Some(normalized)
                }
            });
        let Some(cookie) = env_cookie.or_else(|| configured_roblosecurity_cookie(settings)) else {
            return Ok(None);
        };
        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), format!(".ROBLOSECURITY={cookie}"));
        let url = format!(
            "https://clientsettings.roblox.com/v2/user-channel?binaryType={}",
            binary_type
        );

        let (status, body) = http_get(&url, &headers)?;
        if !(200..300).contains(&status) {
            return Ok(None);
        }

        let parsed = serde_json::from_str::<UserChannel>(&body).map_err(|err| {
            DomainError::Serialization(format!("failed to parse user channel response: {err}"))
        })?;
        Ok(Some(parsed))
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}

impl BootstrapRuntime for FilesystemBootstrapRuntime {
    fn check_connectivity(&self) -> Result<()> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Ruststrap/0.1")
            .build()
            .map_err(|e| DomainError::Network(format!("client build failed: {e}")))?;

        let response = client.get("https://clientsettingscdn.roblox.com").send();

        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(DomainError::Network(format!(
                "connectivity check failed: {e}"
            ))),
        }
    }

    fn resolve_version(&self, mode: LaunchMode, _force_upgrade: bool) -> Result<String> {
        let version = self.check_updates_for_mode(mode)?;
        Ok(version.version_guid)
    }

    fn sync_packages(&self, version_guid: &str) -> Result<()> {
        let mode = self.infer_mode_for_version(version_guid)?;
        let package_map = self.package_map_for_mode(mode)?;
        let mut roblox_state = self.load_roblox_state()?;

        // fetch the package manifest from the deployment server (with fallback)
        let mut manifest_url =
            format!("https://setup.rbxcdn.com/{version_guid}-rbxPkgManifest.txt");
        let empty_headers = HashMap::new();
        let (mut status, mut manifest_body) = http_get(&manifest_url, &empty_headers)?;

        if status == 403 || status == 404 {
            // fallback to /channel/common/ (common for many deployment channels)
            manifest_url = format!(
                "https://setup.rbxcdn.com/channel/common/{version_guid}-rbxPkgManifest.txt"
            );
            let result = http_get(&manifest_url, &empty_headers)?;
            status = result.0;
            manifest_body = result.1;
        }

        if !(200..300).contains(&status) {
            return Err(DomainError::Network(format!(
                "failed to fetch package manifest: HTTP {status} (last tried: {manifest_url})"
            )));
        }

        let packages = parse_package_manifest(&manifest_body)?;

        // download and extract each package
        fs::create_dir_all(&self.config.downloads_dir)?;
        fs::create_dir_all(self.config.versions_dir.join(version_guid))?;

        let mut new_hashes = HashMap::new();

        // preserve the base URL that worked for downloads
        let base_url = if manifest_url.contains("/channel/common/") {
            format!("https://setup.rbxcdn.com/channel/common/{version_guid}-")
        } else {
            format!("https://setup.rbxcdn.com/{version_guid}-")
        };

        for package in &packages {
            // skip ignored packages
            if package.name == "WebView2RuntimeInstaller.zip" {
                continue;
            }

            let archive_filename = format!("{}.zip", package.signature);
            let archive_path = self.config.downloads_dir.join(&archive_filename);
            let download_url = format!("{}{}", base_url, package.name);

            // skip download if we already have this specific signature cached and verified
            let mut skip_download = false;
            if archive_path.exists() {
                // best effort: if it opens, or we trust the filesystem
                skip_download = true;
            }

            if !skip_download {
                http_download(&download_url, &archive_path)?;
            }

            // extract to target directory with retries on corruption
            let package_directory = package_map.get(&package.name).cloned().unwrap_or_default();
            let destination = self
                .config
                .versions_dir
                .join(version_guid)
                .join(normalize_relative_path(&package_directory));

            let mut success = false;
            for attempt in 0..3 {
                if let Err(e) = extract_archive_native(&archive_path, &destination) {
                    if attempt < 2 {
                        // delete corrupted file and retry download
                        if archive_path.exists() {
                            let _ = fs::remove_file(&archive_path);
                        }
                        let download_url = format!("{}{}", base_url, package.name);
                        if let Err(dl_err) = http_download(&download_url, &archive_path) {
                            if attempt == 2 {
                                return Err(dl_err);
                            }
                        }
                    } else {
                        return Err(e);
                    }
                } else {
                    success = true;
                    break;
                }
            }
            if !success {
                return Err(DomainError::Zip(format!(
                    "Failed to extract {} after retries",
                    package.name
                )));
            }

            new_hashes.insert(package.name.clone(), package.signature.clone());
        }

        // write AppSettings.xml
        let app_settings = r#"<?xml version="1.0" encoding="UTF-8"?>
<Settings>
	<ContentFolder>content</ContentFolder>
	<BaseUrl>http://www.roblox.com</BaseUrl>
</Settings>"#;
        fs::write(
            self.config
                .versions_dir
                .join(version_guid)
                .join("AppSettings.xml"),
            app_settings,
        )?;

        // update package hashes in state
        match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => {
                roblox_state.studio.package_hashes = new_hashes;
            }
            _ => {
                roblox_state.player.package_hashes = new_hashes;
            }
        }
        self.save_roblox_state(&roblox_state)?;

        Ok(())
    }

    fn apply_modifications(&self, version_guid: &str) -> Result<()> {
        let version_path = self.config.versions_dir.join(version_guid);
        if !version_path.exists() {
            return Err(DomainError::StateMigration(format!(
                "version directory does not exist: {}",
                version_path.display()
            )));
        }

        let mode = self.infer_mode_for_version(version_guid)?;
        let package_map = self.package_map_for_mode(mode)?;
        let mut state = self.load_roblox_state()?;
        let package_hashes = match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => state.studio.package_hashes.clone(),
            _ => state.player.package_hashes.clone(),
        };
        let mut restored = HashSet::<String>::new();

        for item in &state.mod_manifest {
            if let Some((package, relative_file)) = resolve_restore_package(item, &package_map) {
                if let Some(signature) = package_hashes.get(&package) {
                    let archive_path = self.config.downloads_dir.join(format!("{}.zip", signature));
                    if archive_path.exists() {
                        let package_dir = package_map.get(&package).cloned().unwrap_or_default();
                        let destination = self
                            .config
                            .versions_dir
                            .join(version_guid)
                            .join(normalize_relative_path(&package_dir));
                        if let Err(_) = extract_selected_files_native(
                            &archive_path,
                            &destination,
                            std::slice::from_ref(&relative_file),
                        ) {
                            if archive_path.exists() {
                                fs::remove_file(&archive_path).ok();
                            }
                        } else {
                            restored.insert(format!("{package}:{relative_file}"));
                        }
                    }
                }
            }
        }

        if !restored.is_empty() {
            state.extra.insert(
                "LastRestoreCandidates".to_string(),
                serde_json::to_value(restored.into_iter().collect::<Vec<String>>()).map_err(
                    |err| {
                        DomainError::Serialization(format!(
                            "failed to serialize restore list: {err}"
                        ))
                    },
                )?,
            );
            self.save_roblox_state(&state)?;
        }

        Ok(())
    }

    fn register_system_state(&self, mode: LaunchMode, version_guid: &str) -> Result<()> {
        let mut state = self.load_roblox_state()?;
        match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => {
                state.studio.version_guid = version_guid.to_string();
            }
            _ => {
                state.player.version_guid = version_guid.to_string();
            }
        }
        self.save_roblox_state(&state)
    }

    fn launch_client(&self, mode: LaunchMode, launch_args: &str) -> Result<u32> {
        let settings = self.load_settings()?;

        if settings.cookie_auto_apply {
            if let Some(cookie) = configured_roblosecurity_cookie(&settings) {
                if let Err(error) = persist_roblosecurity_cookie(&cookie) {
                    log::warn!("failed to apply configured Roblox cookie before launch: {error}");
                } else {
                    std::env::set_var("Ruststrap_ROBLOSECURITY", &cookie);
                }
            }
        }

        // record in state
        let mut state = self.load_state()?;
        state.extra.insert(
            "LastLaunch".to_string(),
            serde_json::json!({
                "mode": format!("{mode:?}"),
                "args": launch_args,
            }),
        );
        self.save_state(&state)?;

        // resolve version GUID
        let roblox_state = self.load_roblox_state()?;
        let version_guid = match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => &roblox_state.studio.version_guid,
            _ => &roblox_state.player.version_guid,
        };

        // determine executable name
        let exe_name = match mode {
            LaunchMode::Studio | LaunchMode::StudioAuth => "RobloxStudioBeta.exe",
            _ => "RobloxPlayerBeta.exe",
        };

        let exe_path = self.config.versions_dir.join(version_guid).join(exe_name);

        if !exe_path.exists() {
            return Err(DomainError::Process(format!(
                "Roblox executable not found: {}",
                exe_path.display()
            )));
        }

        // error 773 Fix (clear read-only on RobloxCookies.dat)
        if settings.error_773_fix {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let cookies = std::path::PathBuf::from(local_app_data)
                    .join("Roblox/LocalStorage/RobloxCookies.dat");
                if cookies.exists() {
                    if let Ok(mut perms) = std::fs::metadata(&cookies).map(|m| m.permissions()) {
                        perms.set_readonly(false);
                        let _ = std::fs::set_permissions(&cookies, perms.clone());
                        perms.set_readonly(true);
                        let _ = std::fs::set_permissions(&cookies, perms);
                    }
                }
            }
        }

        // multi-Instance Watcher (runs in a detached background thread)
        if settings.multi_instance_launching && mode == LaunchMode::Player {
            std::thread::spawn(|| {
                crate::multi_instance_watcher::run();
            });
            // small sleep to ensure the watcher's mutex is created before we launch Roblox
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        let integrations = parse_custom_integrations(&settings.custom_integrations);
        let mut autoclose_pids = Vec::<u32>::new();

        for integration in integrations.iter().filter(|entry| entry.pre_launch) {
            if let Some(pid) = launch_custom_integration_process(integration) {
                if integration.auto_close {
                    autoclose_pids.push(pid);
                }
            }
        }

        // build arguments
        let mut args: Vec<String> = Vec::new();
        if !launch_args.is_empty() {
            args.push(launch_args.to_string());
        }

        // auto Close Crash Handler
        if settings.auto_close_crash_handler {
            std::thread::spawn(|| {
                // poll for up to ~10 minutes
                for _ in 0..120 {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let mut command = std::process::Command::new("taskkill");
                    configure_hidden(&mut command);
                    let _ = command
                        .args(["/F", "/IM", "RobloxCrashHandler.exe"])
                        .output();
                }
            });
        }

        let priority_flag = priority_flag_from_selection(settings.selected_process_priority);

        // re-assert protocol ownership on launch so Roblox updates do not permanently
        // steal roblox:// handlers away from Ruststrap.
        if let Ok(current_exe) = std::env::current_exe() {
            let _ = crate::installer::ensure_protocol_ownership_for_exe(&current_exe);
        }

        // launch the process
        let handle = {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let mut cmd = Command::new(&exe_path);
                cmd.args(&args);
                if priority_flag != 0 {
                    cmd.creation_flags(priority_flag);
                }
                cmd.spawn().map_err(|e| {
                    DomainError::Process(format!("failed to launch {}: {e}", exe_path.display()))
                })?
            }
            #[cfg(not(windows))]
            {
                Command::new(&exe_path).args(&args).spawn().map_err(|e| {
                    DomainError::Process(format!("failed to launch {}: {e}", exe_path.display()))
                })?
            }
        };

        for integration in integrations.iter().filter(|entry| !entry.pre_launch) {
            if let Some(pid) = launch_custom_integration_process(integration) {
                if integration.auto_close {
                    autoclose_pids.push(pid);
                }
            }
        }

        let mut state = self.load_state()?;
        state.extra.insert(
            "LastAutoclosePids".to_string(),
            serde_json::to_value(&autoclose_pids).map_err(|err| {
                DomainError::Serialization(format!("failed to serialize autoclose pids: {err}"))
            })?,
        );
        self.save_state(&state)?;

        Ok(handle.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_request_adds_channel_token_header() {
        let request = compose_version_request("WindowsPlayer", Some("znext"), Some("secret-token"));
        assert_eq!(
            request
                .headers
                .get("Roblox-Channel-Token")
                .map(String::as_str),
            Some("secret-token")
        );
        assert!(request.url.contains("/channel/znext"));
    }

    #[test]
    fn compose_lookup_request_uses_version_query_parameter() {
        let request = compose_version_lookup_request("WindowsPlayer", "0.714.0.7141083");
        assert_eq!(
            request.url,
            "https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer?version=0.714.0.7141083"
        );
    }

    #[test]
    fn deployment_override_accepts_bare_hash() {
        let parsed = parse_deployment_version_override("6776addb8fbc4d17")
            .expect("parse override")
            .expect("override value");

        assert_eq!(
            parsed,
            DeploymentVersionOverride::VersionGuid("version-6776addb8fbc4d17".to_string())
        );
    }

    #[test]
    fn deployment_override_accepts_direct_version_number() {
        let parsed = parse_deployment_version_override("0.714.0.7141083")
            .expect("parse override")
            .expect("override value");

        assert_eq!(
            parsed,
            DeploymentVersionOverride::ClientVersionNumber("0.714.0.7141083".to_string())
        );
    }

    #[test]
    fn deployment_override_rejects_invalid_value() {
        let result = parse_deployment_version_override("not-a-valid-version");
        assert!(result.is_err());
    }

    #[test]
    fn enrollment_prefers_private_channel_when_assignment_is_not_one() {
        let user_channel = UserChannel {
            channel: "private-canary".to_string(),
            assignment_type: Some(2),
            token: Some("private-token".to_string()),
        };
        let enrolled = resolve_channel_enrollment(
            "production",
            Some("roblox-player:1+channel:public"),
            None,
            Some(&user_channel),
            None,
            ChannelChangeModeCompat::Automatic,
        );

        assert_eq!(enrolled.channel, "private-canary");
        assert_eq!(enrolled.channel_token.as_deref(), Some("private-token"));
        assert!(enrolled.launch_args.contains("channel:private-canary"));
    }

    #[test]
    fn resolve_restore_package_uses_longest_directory_prefix() {
        let mut package_map = HashMap::new();
        package_map.insert(
            "content-fonts.zip".to_string(),
            "content\\fonts\\".to_string(),
        );
        package_map.insert(
            "content-platform-fonts.zip".to_string(),
            "PlatformContent\\pc\\fonts\\".to_string(),
        );

        let restore = resolve_restore_package("content\\fonts\\families\\Arial.json", &package_map)
            .expect("restore candidate");
        assert_eq!(restore.0, "content-fonts.zip");
        assert_eq!(restore.1, "families\\Arial.json");
    }

    #[test]
    fn process_priority_mapping_matches_ui_order() {
        assert_eq!(priority_flag_from_selection(0), 0x00000040); // low
        assert_eq!(priority_flag_from_selection(1), 0x00004000); // below normal
        assert_eq!(priority_flag_from_selection(2), 0); // normal
        assert_eq!(priority_flag_from_selection(3), 0x00008000); // above normal
        assert_eq!(priority_flag_from_selection(4), 0x00000080); // high
        assert_eq!(priority_flag_from_selection(5), 0x00000100); // realtime
    }
}
