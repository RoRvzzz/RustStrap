/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
pub mod activity_data;
pub mod activity_watcher;
pub mod bootstrapper;
pub mod cleaner;
pub mod commands;
pub mod cookies;
pub mod discord_rpc;
pub mod enums;
pub mod errors;
pub mod events;
pub mod fast_flags;
pub mod game_join;
pub mod global_settings;
pub mod installer;
pub mod launch_flags;
pub mod launch_handler;
pub mod launch_settings;
pub mod logger;
pub mod models;
pub mod multi_instance_watcher;
pub mod orchestrator;
pub mod persistence;
pub mod process_utils;
pub mod region_selector;
pub mod remote_data;
pub mod roblox_api;
#[path = "runtime_stable.rs"]
pub mod runtime;
pub mod self_update;
pub mod watcher;
pub mod window_manipulation;

pub use bootstrapper::{context_from_launch_settings, execute_bootstrap};
pub use commands::{Command, LaunchRequest, LaunchTarget};
pub use cookies::{AuthenticatedUser, CookieState, CookiesManager};
pub use errors::{DomainError, Result};
pub use events::{DomainEvent, PromptKind, WatcherEvent};
pub use fast_flags::FastFlagManager;
pub use installer::{
    check_install_location, cleanup_versions_folder, do_install, do_uninstall,
    do_uninstall_for_reinstall, ensure_protocol_ownership_for_exe, installed_app_path,
    runtime_readiness, RuntimeReadiness,
};
pub use launch_flags::{LaunchFlag, LaunchMode};
pub use launch_settings::ParsedLaunchSettings;
pub use models::{
    BootstrapperStyle, ChannelChangeMode, CleanerOptions, RobloxState, Settings, State,
};
pub use orchestrator::{
    run_bootstrap_flow, BootstrapContext, BootstrapReport, BootstrapRuntime, BootstrapStep,
};
pub use persistence::{
    parse_roblox_state_json, parse_settings_json, parse_state_json, to_pretty_json, AppStateCompat,
    BootstrapperStyleCompat, ChannelChangeModeCompat, CleanerOptionsCompat, RobloxStateFileCompat,
    SettingsFileCompat, StateFileCompat, WindowStateCompat,
};
pub use runtime::{
    build_package_map, compose_version_request, parse_package_manifest, resolve_restore_package,
    BootstrapRuntimeConfig, ClientVersionInfo, FilesystemBootstrapRuntime, PackageEntry,
    VersionRequestSpec,
};
pub use self_update::{check_for_updates, compare_semver, download_update, launch_update};
pub use watcher::{decode_watcher_data, encode_watcher_data, Watcher, WatcherData};

pub use activity_data::ActivityData;
pub use activity_watcher::{
    find_recent_player_log_file, newest_player_log_file, wait_for_recent_player_log, ActivityEvent,
    ActivityState, ActivityWatcher,
};
pub use cleaner::{run_cleaner, CleanerAge, CleanerConfig, CleanerReport};
pub use discord_rpc::{
    fetch_thumbnail_url, fetch_universe_details, fetch_user_display, query_server_location,
    DiscordRichPresence, PresenceButton, PresenceData, RobloxUserDisplay, RpcDisplaySettings,
    DISCORD_APP_ID,
};
pub use enums::*;
pub use game_join::{parse_launch_command, GameJoinData};
pub use global_settings::GlobalSettingsManager;
pub use launch_handler::{
    check_wmf_available, is_roblox_running, kill_background_updater, launch_background_updater,
    launch_multi_instance_watcher, launch_settings, launch_trayhost_process,
    launch_watcher_process, open_url,
};
pub use logger::Logger;
pub use region_selector::{
    region_selector_datacenters, region_selector_join, region_selector_search_games,
    region_selector_servers, region_selector_status, RegionDatacenters, RegionGameSearchEntry,
    RegionSelectorStatus, RegionServerEntry, RegionServerPage,
};
pub use remote_data::RemoteDataManager;
pub use roblox_api::*;
pub use window_manipulation::{
    apply_borderless_fullscreen, get_window_title, set_window_icon, set_window_title,
};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn settings_round_trip_serializes_cleanly() {
        let settings = Settings {
            schema_version: 3,
            locale: "en-GB".to_string(),
            allow_cookie_access: true,
            static_directory: false,
            check_for_updates: true,
            use_fast_flag_manager: false,
            confirm_launches: true,
            bootstrapper_style: BootstrapperStyle::FluentAeroDialog,
            channel_change_mode: ChannelChangeMode::Prompt,
            cleaner_options: CleanerOptions::OnExit,
            enable_activity_tracking: true,
        };

        let json = serde_json::to_string_pretty(&settings).expect("serialize settings");
        let decoded: Settings = serde_json::from_str(&json).expect("deserialize settings");

        assert_eq!(settings, decoded);
    }

    #[test]
    fn state_round_trip_serializes_cleanly() {
        let state = State {
            schema_version: 2,
            force_reinstall: true,
            install_location: Some(r"C:\Program Files\Ruststrap".to_string()),
            estimated_size_kib: 2048,
            last_update_check_utc: Some("2026-03-23T23:15:00Z".to_string()),
        };

        let json = serde_json::to_string(&state).expect("serialize state");
        let decoded: State = serde_json::from_str(&json).expect("deserialize state");

        assert_eq!(state, decoded);
    }

    #[test]
    fn roblox_state_round_trip_serializes_cleanly() {
        let mut player_package_hashes = HashMap::new();
        player_package_hashes.insert("content".to_string(), "abc123".to_string());

        let mut studio_package_hashes = HashMap::new();
        studio_package_hashes.insert("platform".to_string(), "def456".to_string());

        let roblox_state = RobloxState {
            schema_version: 4,
            mod_manifest: vec![
                "content/fonts/families/Arial.json".to_string(),
                "PlatformContent/pc/rbxfont.json".to_string(),
            ],
            player: models::ClientState {
                version_guid: Some("player-version-guid".to_string()),
                estimated_size_kib: 10_240,
                package_hashes: player_package_hashes,
            },
            studio: models::ClientState {
                version_guid: Some("studio-version-guid".to_string()),
                estimated_size_kib: 20_480,
                package_hashes: studio_package_hashes,
            },
        };

        let json = serde_json::to_value(&roblox_state).expect("serialize roblox state");
        let decoded: RobloxState = serde_json::from_value(json).expect("deserialize roblox state");

        assert_eq!(roblox_state, decoded);
    }

    #[test]
    fn command_and_event_types_have_stable_shapes() {
        let command = Command::LaunchPlayer {
            args: vec!["-game".to_string()],
        };

        let event = DomainEvent::Progress {
            current: 42,
            total: 100,
        };

        let command_json = serde_json::to_value(&command).expect("serialize command");
        let event_json = serde_json::to_value(&event).expect("serialize event");

        assert!(command_json.is_object());
        assert!(event_json.is_object());
    }
}
