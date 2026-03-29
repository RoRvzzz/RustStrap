/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::errors::{DomainError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapperStyleCompat {
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

impl Default for BootstrapperStyleCompat {
    fn default() -> Self {
        Self::FluentAeroDialog
    }
}

impl BootstrapperStyleCompat {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::VistaDialog => 0,
            Self::LegacyDialog2008 => 1,
            Self::LegacyDialog2011 => 2,
            Self::ProgressDialog => 3,
            Self::ClassicFluentDialog => 4,
            Self::TwentyFiveDialog => 5,
            Self::ByfronDialog => 6,
            Self::FluentDialog => 7,
            Self::FluentAeroDialog => 8,
            Self::CustomDialog => 9,
        }
    }

    fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::VistaDialog,
            1 => Self::LegacyDialog2008,
            2 => Self::LegacyDialog2011,
            3 => Self::ProgressDialog,
            4 => Self::ClassicFluentDialog,
            5 => Self::TwentyFiveDialog,
            6 => Self::ByfronDialog,
            7 => Self::FluentDialog,
            8 => Self::FluentAeroDialog,
            9 => Self::CustomDialog,
            _ => return None,
        })
    }

    fn from_name(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "vistadialog" | "vista_dialog" => Some(Self::VistaDialog),
            "legacydialog2008" | "legacy_dialog_2008" => Some(Self::LegacyDialog2008),
            "legacydialog2011" | "legacy_dialog_2011" => Some(Self::LegacyDialog2011),
            "progressdialog" | "progress_dialog" => Some(Self::ProgressDialog),
            "classicfluentdialog" | "classic_fluent_dialog" => Some(Self::ClassicFluentDialog),
            "twentyfivedialog" | "twenty_five_dialog" => Some(Self::TwentyFiveDialog),
            "byfrondialog" | "byfron_dialog" => Some(Self::ByfronDialog),
            "fluentdialog" | "fluent_dialog" | "Ruststrap" => Some(Self::FluentDialog),
            "fluentaerodialog" | "fluent_aero_dialog" => Some(Self::FluentAeroDialog),
            "customdialog" | "custom_dialog" => Some(Self::CustomDialog),
            _ => None,
        }
    }
}

impl Serialize for BootstrapperStyleCompat {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.as_i32())
    }
}

impl<'de> Deserialize<'de> for BootstrapperStyleCompat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Number(i32),
            Name(String),
        }

        let wire = Wire::deserialize(deserializer)?;
        match wire {
            Wire::Number(value) => BootstrapperStyleCompat::from_i32(value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid BootstrapperStyle value {value}"))
            }),
            Wire::Name(value) => BootstrapperStyleCompat::from_name(&value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid BootstrapperStyle name {value}"))
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelChangeModeCompat {
    Automatic,
    Prompt,
    Ignore,
}

impl Default for ChannelChangeModeCompat {
    fn default() -> Self {
        Self::Automatic
    }
}

impl ChannelChangeModeCompat {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Automatic => 0,
            Self::Prompt => 1,
            Self::Ignore => 2,
        }
    }

    fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::Automatic,
            1 => Self::Prompt,
            2 => Self::Ignore,
            _ => return None,
        })
    }

    fn from_name(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "automatic" => Some(Self::Automatic),
            "prompt" => Some(Self::Prompt),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

impl Serialize for ChannelChangeModeCompat {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.as_i32())
    }
}

impl<'de> Deserialize<'de> for ChannelChangeModeCompat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Number(i32),
            Name(String),
        }

        let wire = Wire::deserialize(deserializer)?;
        match wire {
            Wire::Number(value) => ChannelChangeModeCompat::from_i32(value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid ChannelChangeMode value {value}"))
            }),
            Wire::Name(value) => ChannelChangeModeCompat::from_name(&value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid ChannelChangeMode name {value}"))
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanerOptionsCompat {
    Never,
    OneDay,
    OneWeek,
    OneMonth,
    TwoMonths,
}

impl Default for CleanerOptionsCompat {
    fn default() -> Self {
        Self::Never
    }
}

impl CleanerOptionsCompat {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Never => 0,
            Self::OneDay => 1,
            Self::OneWeek => 2,
            Self::OneMonth => 3,
            Self::TwoMonths => 4,
        }
    }

    fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::Never,
            1 => Self::OneDay,
            2 => Self::OneWeek,
            3 => Self::OneMonth,
            4 => Self::TwoMonths,
            _ => return None,
        })
    }

    fn from_name(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "never" => Some(Self::Never),
            "oneday" | "one_day" => Some(Self::OneDay),
            "oneweek" | "one_week" => Some(Self::OneWeek),
            "onemonth" | "one_month" => Some(Self::OneMonth),
            "twomonths" | "two_months" => Some(Self::TwoMonths),
            _ => None,
        }
    }
}

impl Serialize for CleanerOptionsCompat {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.as_i32())
    }
}

impl<'de> Deserialize<'de> for CleanerOptionsCompat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Number(i32),
            Name(String),
        }

        let wire = Wire::deserialize(deserializer)?;
        match wire {
            Wire::Number(value) => CleanerOptionsCompat::from_i32(value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid CleanerOptions value {value}"))
            }),
            Wire::Name(value) => CleanerOptionsCompat::from_name(&value).ok_or_else(|| {
                serde::de::Error::custom(format!("invalid CleanerOptions name {value}"))
            }),
        }
    }
}

// serde default helpers for Froststrap fields
fn default_true() -> bool {
    true
}
fn default_multiblox_count() -> i32 {
    2
}
fn default_multiblox_delay() -> i32 {
    1500
}
fn default_process_priority() -> i32 {
    2
} // Normal

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsFileCompat {
    #[serde(rename = "AllowCookieAccess", alias = "allow_cookie_access")]
    pub allow_cookie_access: bool,
    #[serde(rename = "CookieAutoApply", alias = "cookie_auto_apply", default)]
    pub cookie_auto_apply: bool,
    #[serde(rename = "CookieRoblosecurity", alias = "cookie_roblosecurity", default)]
    pub cookie_roblosecurity: String,
    #[serde(rename = "BootstrapperStyle", alias = "bootstrapper_style")]
    pub bootstrapper_style: BootstrapperStyleCompat,
    #[serde(rename = "BootstrapperIcon", alias = "bootstrapper_icon")]
    pub bootstrapper_icon: i32,
    #[serde(rename = "BootstrapperTitle", alias = "bootstrapper_title")]
    pub bootstrapper_title: String,
    #[serde(
        rename = "BootstrapperIconCustomLocation",
        alias = "bootstrapper_icon_custom_location"
    )]
    pub bootstrapper_icon_custom_location: String,
    #[serde(rename = "RobloxIcon", alias = "roblox_icon")]
    pub roblox_icon: i32,
    #[serde(rename = "RobloxTitle", alias = "roblox_title")]
    pub roblox_title: String,
    #[serde(
        rename = "RobloxIconCustomLocation",
        alias = "roblox_icon_custom_location"
    )]
    pub roblox_icon_custom_location: String,
    #[serde(rename = "Theme", alias = "theme")]
    pub theme: i32,
    #[serde(rename = "DeveloperMode", alias = "developer_mode")]
    pub developer_mode: bool,
    #[serde(rename = "ForceLocalData", alias = "force_local_data")]
    pub force_local_data: bool,
    #[serde(rename = "CheckForUpdates", alias = "check_for_updates")]
    pub check_for_updates: bool,
    #[serde(rename = "MultiInstanceLaunching", alias = "multi_instance_launching")]
    pub multi_instance_launching: bool,
    #[serde(rename = "ConfirmLaunches", alias = "confirm_launches")]
    pub confirm_launches: bool,
    #[serde(rename = "Locale", alias = "locale")]
    pub locale: String,
    #[serde(rename = "ForceRobloxLanguage", alias = "force_roblox_language")]
    pub force_roblox_language: bool,
    #[serde(rename = "UseFastFlagManager", alias = "use_fast_flag_manager")]
    pub use_fast_flag_manager: bool,
    #[serde(rename = "WPFSoftwareRender", alias = "wpf_software_render")]
    pub wpf_software_render: bool,
    #[serde(rename = "EnableAnalytics", alias = "enable_analytics")]
    pub enable_analytics: bool,
    #[serde(rename = "UpdateRoblox", alias = "update_roblox")]
    pub update_roblox: bool,
    #[serde(rename = "StaticDirectory", alias = "static_directory")]
    pub static_directory: bool,
    #[serde(rename = "Channel", alias = "channel")]
    pub channel: String,
    #[serde(rename = "ChannelChangeMode", alias = "channel_change_mode")]
    pub channel_change_mode: ChannelChangeModeCompat,
    #[serde(rename = "ChannelHash", alias = "channel_hash")]
    pub channel_hash: String,
    #[serde(
        rename = "DownloadingStringFormat",
        alias = "downloading_string_format"
    )]
    pub downloading_string_format: String,
    #[serde(rename = "SelectedCustomTheme", alias = "selected_custom_theme")]
    pub selected_custom_theme: Option<String>,
    #[serde(
        rename = "BackgroundUpdatesEnabled",
        alias = "background_updates_enabled"
    )]
    pub background_updates_enabled: bool,
    #[serde(
        rename = "DebugDisableVersionPackageCleanup",
        alias = "debug_disable_version_package_cleanup"
    )]
    pub debug_disable_version_package_cleanup: bool,
    #[serde(
        rename = "EnableBetterMatchmaking",
        alias = "enable_better_matchmaking"
    )]
    pub enable_better_matchmaking: bool,
    #[serde(
        rename = "EnableBetterMatchmakingRandomization",
        alias = "enable_better_matchmaking_randomization"
    )]
    pub enable_better_matchmaking_randomization: bool,
    #[serde(rename = "WebEnvironment", alias = "web_environment")]
    pub web_environment: String,
    #[serde(rename = "CleanerOptions", alias = "cleaner_options")]
    pub cleaner_options: CleanerOptionsCompat,
    #[serde(rename = "CleanerDirectories", alias = "cleaner_directories")]
    pub cleaner_directories: Vec<String>,
    #[serde(
        rename = "FakeBorderlessFullscreen",
        alias = "fake_borderless_fullscreen"
    )]
    pub fake_borderless_fullscreen: bool,
    #[serde(rename = "EnableActivityTracking", alias = "enable_activity_tracking")]
    pub enable_activity_tracking: bool,
    #[serde(rename = "UseDiscordRichPresence", alias = "use_discord_rich_presence")]
    pub use_discord_rich_presence: bool,
    #[serde(rename = "HideRPCButtons", alias = "hide_rpc_buttons")]
    pub hide_rpc_buttons: bool,
    #[serde(
        rename = "ShowAccountOnRichPresence",
        alias = "show_account_on_rich_presence"
    )]
    pub show_account_on_rich_presence: bool,
    #[serde(rename = "ShowServerDetails", alias = "show_server_details")]
    pub show_server_details: bool,
    #[serde(rename = "CustomIntegrations", alias = "custom_integrations")]
    pub custom_integrations: Vec<Value>,
    #[serde(rename = "UseDisableAppPatch", alias = "use_disable_app_patch")]
    pub use_disable_app_patch: bool,

    // ── froststrap-exclusive fields ──
    #[serde(rename = "AutoRejoin", alias = "auto_rejoin", default)]
    pub auto_rejoin: bool,
    #[serde(
        rename = "PlaytimeCounter",
        alias = "playtime_counter",
        default = "default_true"
    )]
    pub playtime_counter: bool,
    #[serde(rename = "ShowServerUptime", alias = "show_server_uptime", default)]
    pub show_server_uptime: bool,
    #[serde(
        rename = "ShowGameHistoryMenu",
        alias = "show_game_history_menu",
        default = "default_true"
    )]
    pub show_game_history_menu: bool,
    #[serde(
        rename = "EnableCustomStatusDisplay",
        alias = "enable_custom_status_display",
        default = "default_true"
    )]
    pub enable_custom_status_display: bool,
    #[serde(
        rename = "ShowUsingRuststrapRPC",
        alias = "show_using_ruststrap_rpc",
        default = "default_true"
    )]
    pub show_using_ruststrap_rpc: bool,
    #[serde(rename = "StudioRPC", alias = "studio_rpc", default)]
    pub studio_rpc: bool,
    #[serde(
        rename = "StudioThumbnailChanging",
        alias = "studio_thumbnail_changing",
        default
    )]
    pub studio_thumbnail_changing: bool,
    #[serde(rename = "StudioEditingInfo", alias = "studio_editing_info", default)]
    pub studio_editing_info: bool,
    #[serde(
        rename = "StudioWorkspaceInfo",
        alias = "studio_workspace_info",
        default
    )]
    pub studio_workspace_info: bool,
    #[serde(rename = "StudioShowTesting", alias = "studio_show_testing", default)]
    pub studio_show_testing: bool,
    #[serde(rename = "StudioGameButton", alias = "studio_game_button", default)]
    pub studio_game_button: bool,
    #[serde(
        rename = "DisableRobloxRecording",
        alias = "disable_roblox_recording",
        default
    )]
    pub disable_roblox_recording: bool,
    #[serde(
        rename = "DisableRobloxScreenshots",
        alias = "disable_roblox_screenshots",
        default
    )]
    pub disable_roblox_screenshots: bool,
    #[serde(
        rename = "AutoCloseCrashHandler",
        alias = "auto_close_crash_handler",
        default
    )]
    pub auto_close_crash_handler: bool,
    #[serde(rename = "Error773Fix", alias = "error_773_fix", default)]
    pub error_773_fix: bool,
    #[serde(
        rename = "MultibloxInstanceCount",
        alias = "multiblox_instance_count",
        default = "default_multiblox_count"
    )]
    pub multiblox_instance_count: i32,
    #[serde(
        rename = "MultibloxDelayMs",
        alias = "multiblox_delay_ms",
        default = "default_multiblox_delay"
    )]
    pub multiblox_delay_ms: i32,
    #[serde(
        rename = "SelectedProcessPriority",
        alias = "selected_process_priority",
        default = "default_process_priority"
    )]
    pub selected_process_priority: i32,
    #[serde(rename = "SelectedRegion", alias = "selected_region", default)]
    pub selected_region: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for SettingsFileCompat {
    fn default() -> Self {
        Self {
            allow_cookie_access: false,
            cookie_auto_apply: false,
            cookie_roblosecurity: String::new(),
            bootstrapper_style: BootstrapperStyleCompat::FluentAeroDialog,
            bootstrapper_icon: 0,
            bootstrapper_title: "Ruststrap".to_string(),
            bootstrapper_icon_custom_location: String::new(),
            roblox_icon: 0,
            roblox_title: "Roblox".to_string(),
            roblox_icon_custom_location: String::new(),
            theme: 0,
            developer_mode: false,
            force_local_data: false,
            check_for_updates: true,
            multi_instance_launching: false,
            confirm_launches: true,
            locale: "nil".to_string(),
            force_roblox_language: false,
            use_fast_flag_manager: true,
            wpf_software_render: false,
            enable_analytics: false,
            update_roblox: true,
            static_directory: false,
            channel: "production".to_string(),
            channel_change_mode: ChannelChangeModeCompat::Automatic,
            channel_hash: String::new(),
            downloading_string_format: "Downloading {0} - {1}MB / {2}MB".to_string(),
            selected_custom_theme: None,
            background_updates_enabled: false,
            debug_disable_version_package_cleanup: false,
            enable_better_matchmaking: false,
            enable_better_matchmaking_randomization: false,
            web_environment: "Production".to_string(),
            cleaner_options: CleanerOptionsCompat::Never,
            cleaner_directories: Vec::new(),
            fake_borderless_fullscreen: false,
            enable_activity_tracking: true,
            use_discord_rich_presence: true,
            hide_rpc_buttons: true,
            show_account_on_rich_presence: false,
            show_server_details: false,
            custom_integrations: Vec::new(),
            use_disable_app_patch: false,

            // froststrap-exclusive defaults
            auto_rejoin: false,
            playtime_counter: true,
            show_server_uptime: false,
            show_game_history_menu: true,
            enable_custom_status_display: true,
            show_using_ruststrap_rpc: true,
            studio_rpc: false,
            studio_thumbnail_changing: false,
            studio_editing_info: false,
            studio_workspace_info: false,
            studio_show_testing: false,
            studio_game_button: false,
            disable_roblox_recording: false,
            disable_roblox_screenshots: false,
            auto_close_crash_handler: false,
            error_773_fix: false,
            multiblox_instance_count: 2,
            multiblox_delay_ms: 1500,
            selected_process_priority: 2,
            selected_region: String::new(),

            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppStateCompat {
    #[serde(rename = "VersionGuid", alias = "version_guid")]
    pub version_guid: String,
    #[serde(rename = "PackageHashes", alias = "package_hashes")]
    pub package_hashes: HashMap<String, String>,
    #[serde(rename = "Size", alias = "size")]
    pub size: i32,
}

impl Default for AppStateCompat {
    fn default() -> Self {
        Self {
            version_guid: String::new(),
            package_hashes: HashMap::new(),
            size: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowStateCompat {
    #[serde(rename = "Width", alias = "width")]
    pub width: f64,
    #[serde(rename = "Height", alias = "height")]
    pub height: f64,
    #[serde(rename = "Left", alias = "left")]
    pub left: f64,
    #[serde(rename = "Top", alias = "top")]
    pub top: f64,
}

impl Default for WindowStateCompat {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            left: 0.0,
            top: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StateFileCompat {
    #[serde(rename = "TestModeWarningShown", alias = "test_mode_warning_shown")]
    pub test_mode_warning_shown: bool,
    #[serde(rename = "IgnoreOutdatedChannel", alias = "ignore_outdated_channel")]
    pub ignore_outdated_channel: bool,
    #[serde(rename = "WatcherRunning", alias = "watcher_running")]
    pub watcher_running: bool,
    #[serde(rename = "PromptWebView2Install", alias = "prompt_web_view2_install")]
    pub prompt_web_view2_install: bool,
    #[serde(rename = "LastPage", alias = "last_page")]
    pub last_page: Option<String>,
    #[serde(rename = "ForceReinstall", alias = "force_reinstall")]
    pub force_reinstall: bool,
    #[serde(rename = "SettingsWindow", alias = "settings_window")]
    pub settings_window: WindowStateCompat,
    #[serde(
        rename = "Player",
        alias = "player",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecated_player: Option<AppStateCompat>,
    #[serde(
        rename = "Studio",
        alias = "studio",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecated_studio: Option<AppStateCompat>,
    #[serde(
        rename = "ModManifest",
        alias = "mod_manifest",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecated_mod_manifest: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for StateFileCompat {
    fn default() -> Self {
        Self {
            test_mode_warning_shown: false,
            ignore_outdated_channel: false,
            watcher_running: false,
            prompt_web_view2_install: true,
            last_page: None,
            force_reinstall: false,
            settings_window: WindowStateCompat::default(),
            deprecated_player: None,
            deprecated_studio: None,
            deprecated_mod_manifest: None,
            extra: BTreeMap::new(),
        }
    }
}

impl StateFileCompat {
    pub fn has_legacy_embedded_roblox_state(&self) -> bool {
        self.deprecated_player.is_some()
            || self.deprecated_studio.is_some()
            || self.deprecated_mod_manifest.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RobloxStateFileCompat {
    #[serde(rename = "Player", alias = "player")]
    pub player: AppStateCompat,
    #[serde(rename = "Studio", alias = "studio")]
    pub studio: AppStateCompat,
    #[serde(rename = "ModManifest", alias = "mod_manifest")]
    pub mod_manifest: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for RobloxStateFileCompat {
    fn default() -> Self {
        Self {
            player: AppStateCompat::default(),
            studio: AppStateCompat::default(),
            mod_manifest: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

pub fn parse_settings_json(input: &str) -> Result<SettingsFileCompat> {
    serde_json::from_str::<SettingsFileCompat>(input)
        .map_err(|err| DomainError::Serialization(format!("settings parse failed: {err}")))
}

pub fn parse_state_json(input: &str) -> Result<StateFileCompat> {
    serde_json::from_str::<StateFileCompat>(input)
        .map_err(|err| DomainError::Serialization(format!("state parse failed: {err}")))
}

pub fn parse_roblox_state_json(input: &str) -> Result<RobloxStateFileCompat> {
    serde_json::from_str::<RobloxStateFileCompat>(input)
        .map_err(|err| DomainError::Serialization(format!("roblox state parse failed: {err}")))
}

pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| DomainError::Serialization(format!("json serialization failed: {err}")))
}
