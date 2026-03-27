use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapperStyle {
    VistaDialog,
    LegacyDialog2008,
    LegacyDialog2011,
    ProgressDialog,
    ClassicFluentDialog,
    TwentyFiveDialog,
    ByfronDialog,
    FluentDialog,
    FluentAeroDialog,
    CustomDialog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelChangeMode {
    Automatic,
    Prompt,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanerOptions {
    Never,
    OnExit,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub locale: String,
    pub allow_cookie_access: bool,
    pub static_directory: bool,
    pub check_for_updates: bool,
    pub use_fast_flag_manager: bool,
    pub confirm_launches: bool,
    pub bootstrapper_style: BootstrapperStyle,
    pub channel_change_mode: ChannelChangeMode,
    pub cleaner_options: CleanerOptions,
    pub enable_activity_tracking: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            locale: "nil".to_string(),
            allow_cookie_access: false,
            static_directory: false,
            check_for_updates: true,
            use_fast_flag_manager: true,
            confirm_launches: false,
            bootstrapper_style: BootstrapperStyle::FluentDialog,
            channel_change_mode: ChannelChangeMode::Automatic,
            cleaner_options: CleanerOptions::Never,
            enable_activity_tracking: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub schema_version: u32,
    pub force_reinstall: bool,
    pub install_location: Option<String>,
    pub estimated_size_kib: u64,
    pub last_update_check_utc: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            force_reinstall: false,
            install_location: None,
            estimated_size_kib: 0,
            last_update_check_utc: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientState {
    pub version_guid: Option<String>,
    pub estimated_size_kib: u64,
    pub package_hashes: HashMap<String, String>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            version_guid: None,
            estimated_size_kib: 0,
            package_hashes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RobloxState {
    pub schema_version: u32,
    pub mod_manifest: Vec<String>,
    pub player: ClientState,
    pub studio: ClientState,
}

impl Default for RobloxState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mod_manifest: Vec::new(),
            player: ClientState::default(),
            studio: ClientState::default(),
        }
    }
}
