use std::collections::HashMap;

use crate::launch_flags::{LaunchFlag, LaunchMode};

const MENU_IDENTIFIERS: &[&str] = &["preferences", "menu", "settings"];
const WATCHER_IDENTIFIERS: &[&str] = &["watcher"];
const TRAYHOST_IDENTIFIERS: &[&str] = &["trayhost"];
const MULTI_INSTANCE_WATCHER_IDENTIFIERS: &[&str] = &["multiinstancewatcher"];
const BACKGROUND_UPDATER_IDENTIFIERS: &[&str] = &["backgroundupdater"];
const QUIET_IDENTIFIERS: &[&str] = &["quiet"];
const UNINSTALL_IDENTIFIERS: &[&str] = &["uninstall"];
const NO_LAUNCH_IDENTIFIERS: &[&str] = &["nolaunch"];
const TEST_MODE_IDENTIFIERS: &[&str] = &["testmode"];
const NO_GPU_IDENTIFIERS: &[&str] = &["nogpu"];
const UPGRADE_IDENTIFIERS: &[&str] = &["upgrade"];
const PLAYER_IDENTIFIERS: &[&str] = &["player"];
const STUDIO_IDENTIFIERS: &[&str] = &["studio"];
const VERSION_IDENTIFIERS: &[&str] = &["version"];
const CHANNEL_IDENTIFIERS: &[&str] = &["channel"];
const FORCE_IDENTIFIERS: &[&str] = &["force"];
const BLOXSHADE_IDENTIFIERS: &[&str] = &["bloxshade"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FlagKey {
    Menu,
    Watcher,
    TrayHost,
    MultiInstanceWatcher,
    BackgroundUpdater,
    Quiet,
    Uninstall,
    NoLaunch,
    TestMode,
    NoGpu,
    Upgrade,
    Player,
    Studio,
    Version,
    Channel,
    Force,
    Bloxshade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLaunchSettings {
    pub menu_flag: LaunchFlag,
    pub watcher_flag: LaunchFlag,
    pub tray_host_flag: LaunchFlag,
    pub multi_instance_watcher_flag: LaunchFlag,
    pub background_updater_flag: LaunchFlag,
    pub quiet_flag: LaunchFlag,
    pub uninstall_flag: LaunchFlag,
    pub no_launch_flag: LaunchFlag,
    pub test_mode_flag: LaunchFlag,
    pub no_gpu_flag: LaunchFlag,
    pub upgrade_flag: LaunchFlag,
    pub player_flag: LaunchFlag,
    pub studio_flag: LaunchFlag,
    pub version_flag: LaunchFlag,
    pub channel_flag: LaunchFlag,
    pub force_flag: LaunchFlag,
    pub bloxshade_flag: LaunchFlag,
    pub roblox_launch_mode: LaunchMode,
    pub roblox_launch_args: String,
    pub args: Vec<String>,
}

impl ParsedLaunchSettings {
    pub fn new(args: Vec<String>) -> Self {
        Self {
            menu_flag: LaunchFlag::new(MENU_IDENTIFIERS),
            watcher_flag: LaunchFlag::new(WATCHER_IDENTIFIERS),
            tray_host_flag: LaunchFlag::new(TRAYHOST_IDENTIFIERS),
            multi_instance_watcher_flag: LaunchFlag::new(MULTI_INSTANCE_WATCHER_IDENTIFIERS),
            background_updater_flag: LaunchFlag::new(BACKGROUND_UPDATER_IDENTIFIERS),
            quiet_flag: LaunchFlag::new(QUIET_IDENTIFIERS),
            uninstall_flag: LaunchFlag::new(UNINSTALL_IDENTIFIERS),
            no_launch_flag: LaunchFlag::new(NO_LAUNCH_IDENTIFIERS),
            test_mode_flag: LaunchFlag::new(TEST_MODE_IDENTIFIERS),
            no_gpu_flag: LaunchFlag::new(NO_GPU_IDENTIFIERS),
            upgrade_flag: LaunchFlag::new(UPGRADE_IDENTIFIERS),
            player_flag: LaunchFlag::new(PLAYER_IDENTIFIERS),
            studio_flag: LaunchFlag::new(STUDIO_IDENTIFIERS),
            version_flag: LaunchFlag::new(VERSION_IDENTIFIERS),
            channel_flag: LaunchFlag::new(CHANNEL_IDENTIFIERS),
            force_flag: LaunchFlag::new(FORCE_IDENTIFIERS),
            bloxshade_flag: LaunchFlag::new(BLOXSHADE_IDENTIFIERS),
            roblox_launch_mode: LaunchMode::None,
            roblox_launch_args: String::new(),
            args,
        }
    }

    pub fn parse(args: &[String]) -> Self {
        let mut parsed = Self::new(args.to_vec());
        let flag_map = build_flag_map();

        let mut start_idx = 0usize;

        if let Some(arg) = parsed.args.first() {
            if arg.starts_with("roblox:") || arg.starts_with("roblox-player:") {
                parsed.roblox_launch_mode = LaunchMode::Player;
                parsed.roblox_launch_args = arg.clone();
                start_idx = 1;
            } else if arg.starts_with("roblox-studio-auth:") {
                parsed.roblox_launch_mode = LaunchMode::StudioAuth;
                parsed.roblox_launch_args = arg.clone();
                start_idx = 1;
            } else if arg.starts_with("roblox-studio:") {
                parsed.roblox_launch_mode = LaunchMode::Studio;
                parsed.roblox_launch_args = arg.clone();
                start_idx = 1;
            } else if arg.starts_with("version-") {
                parsed.version_flag.mark_active(Some(arg.clone()));
                start_idx = 1;
            }
        }

        let mut i = start_idx;
        while i < parsed.args.len() {
            let arg = parsed.args[i].clone();
            if !arg.starts_with('-') {
                i += 1;
                continue;
            }

            let identifier = arg.trim_start_matches('-').to_ascii_lowercase();
            let Some(flag_key) = flag_map.get(identifier.as_str()).copied() else {
                i += 1;
                continue;
            };

            if parsed.flag(flag_key).active {
                i += 1;
                continue;
            }

            let mut data = None;
            if i + 1 < parsed.args.len() {
                let next = parsed.args[i + 1].clone();
                if !next.starts_with('-') {
                    data = Some(next);
                    i += 1;
                }
            }

            parsed.flag_mut(flag_key).mark_active(data);
            i += 1;
        }

        if parsed.version_flag.active {
            parsed.roblox_launch_mode = LaunchMode::Unknown;
        }

        if parsed.player_flag.active {
            parsed.parse_player(parsed.player_flag.data.clone());
        } else if parsed.studio_flag.active {
            parsed.parse_studio(parsed.studio_flag.data.clone());
        }

        parsed
    }

    pub fn bypass_update_check(&self, is_debug_build: bool) -> bool {
        if is_debug_build {
            true
        } else {
            self.uninstall_flag.active || self.watcher_flag.active
        }
    }

    fn parse_player(&mut self, data: Option<String>) {
        self.roblox_launch_mode = LaunchMode::Player;
        if let Some(value) = data.filter(|v| !v.is_empty()) {
            self.roblox_launch_args = value;
        }
    }

    fn parse_studio(&mut self, data: Option<String>) {
        self.roblox_launch_mode = LaunchMode::Studio;

        let Some(value) = data.filter(|v| !v.is_empty()) else {
            return;
        };

        if value.starts_with("roblox-studio:") {
            self.roblox_launch_args = value;
        } else if value.starts_with("roblox-studio-auth:") {
            self.roblox_launch_mode = LaunchMode::StudioAuth;
            self.roblox_launch_args = value;
        } else {
            self.roblox_launch_args = format!("-task EditFile -localPlaceFile \"{value}\"");
        }
    }

    fn flag(&self, key: FlagKey) -> &LaunchFlag {
        match key {
            FlagKey::Menu => &self.menu_flag,
            FlagKey::Watcher => &self.watcher_flag,
            FlagKey::TrayHost => &self.tray_host_flag,
            FlagKey::MultiInstanceWatcher => &self.multi_instance_watcher_flag,
            FlagKey::BackgroundUpdater => &self.background_updater_flag,
            FlagKey::Quiet => &self.quiet_flag,
            FlagKey::Uninstall => &self.uninstall_flag,
            FlagKey::NoLaunch => &self.no_launch_flag,
            FlagKey::TestMode => &self.test_mode_flag,
            FlagKey::NoGpu => &self.no_gpu_flag,
            FlagKey::Upgrade => &self.upgrade_flag,
            FlagKey::Player => &self.player_flag,
            FlagKey::Studio => &self.studio_flag,
            FlagKey::Version => &self.version_flag,
            FlagKey::Channel => &self.channel_flag,
            FlagKey::Force => &self.force_flag,
            FlagKey::Bloxshade => &self.bloxshade_flag,
        }
    }

    fn flag_mut(&mut self, key: FlagKey) -> &mut LaunchFlag {
        match key {
            FlagKey::Menu => &mut self.menu_flag,
            FlagKey::Watcher => &mut self.watcher_flag,
            FlagKey::TrayHost => &mut self.tray_host_flag,
            FlagKey::MultiInstanceWatcher => &mut self.multi_instance_watcher_flag,
            FlagKey::BackgroundUpdater => &mut self.background_updater_flag,
            FlagKey::Quiet => &mut self.quiet_flag,
            FlagKey::Uninstall => &mut self.uninstall_flag,
            FlagKey::NoLaunch => &mut self.no_launch_flag,
            FlagKey::TestMode => &mut self.test_mode_flag,
            FlagKey::NoGpu => &mut self.no_gpu_flag,
            FlagKey::Upgrade => &mut self.upgrade_flag,
            FlagKey::Player => &mut self.player_flag,
            FlagKey::Studio => &mut self.studio_flag,
            FlagKey::Version => &mut self.version_flag,
            FlagKey::Channel => &mut self.channel_flag,
            FlagKey::Force => &mut self.force_flag,
            FlagKey::Bloxshade => &mut self.bloxshade_flag,
        }
    }
}

fn build_flag_map() -> HashMap<&'static str, FlagKey> {
    let mut map = HashMap::new();

    for identifier in MENU_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Menu);
    }
    for identifier in WATCHER_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Watcher);
    }
    for identifier in TRAYHOST_IDENTIFIERS {
        map.insert(*identifier, FlagKey::TrayHost);
    }
    for identifier in MULTI_INSTANCE_WATCHER_IDENTIFIERS {
        map.insert(*identifier, FlagKey::MultiInstanceWatcher);
    }
    for identifier in BACKGROUND_UPDATER_IDENTIFIERS {
        map.insert(*identifier, FlagKey::BackgroundUpdater);
    }
    for identifier in QUIET_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Quiet);
    }
    for identifier in UNINSTALL_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Uninstall);
    }
    for identifier in NO_LAUNCH_IDENTIFIERS {
        map.insert(*identifier, FlagKey::NoLaunch);
    }
    for identifier in TEST_MODE_IDENTIFIERS {
        map.insert(*identifier, FlagKey::TestMode);
    }
    for identifier in NO_GPU_IDENTIFIERS {
        map.insert(*identifier, FlagKey::NoGpu);
    }
    for identifier in UPGRADE_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Upgrade);
    }
    for identifier in PLAYER_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Player);
    }
    for identifier in STUDIO_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Studio);
    }
    for identifier in VERSION_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Version);
    }
    for identifier in CHANNEL_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Channel);
    }
    for identifier in FORCE_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Force);
    }
    for identifier in BLOXSHADE_IDENTIFIERS {
        map.insert(*identifier, FlagKey::Bloxshade);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn as_args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_raw_roblox_url_as_player_launch() {
        let parsed =
            ParsedLaunchSettings::parse(&as_args(&["roblox://experiences/start?placeId=123"]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::Player);
        assert!(parsed
            .roblox_launch_args
            .starts_with("roblox://experiences/start?"));
    }

    #[test]
    fn parses_raw_roblox_player_url_as_player_launch() {
        let parsed = ParsedLaunchSettings::parse(&as_args(&[
            "roblox-player:1+launchmode:play+placelauncherurl:https%3A%2F%2Fexample.com",
        ]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::Player);
        assert!(parsed.roblox_launch_args.starts_with("roblox-player:"));
    }

    #[test]
    fn parses_raw_roblox_studio_url_as_studio_launch() {
        let parsed = ParsedLaunchSettings::parse(&as_args(&["roblox-studio:1+launchmode:edit"]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::Studio);
        assert!(parsed.roblox_launch_args.starts_with("roblox-studio:"));
    }

    #[test]
    fn parses_player_flag_with_protocol_payload() {
        let parsed = ParsedLaunchSettings::parse(&as_args(&[
            "-player",
            "roblox://experiences/start?placeId=456",
        ]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::Player);
        assert_eq!(
            parsed.roblox_launch_args,
            "roblox://experiences/start?placeId=456"
        );
    }

    #[test]
    fn parses_studio_protocol_launch() {
        let parsed =
            ParsedLaunchSettings::parse(&as_args(&["-studio", "roblox-studio:1+launchmode:edit"]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::Studio);
        assert!(parsed.roblox_launch_args.starts_with("roblox-studio:"));
    }

    #[test]
    fn parses_studio_auth_protocol_launch() {
        let parsed =
            ParsedLaunchSettings::parse(&as_args(&["-studio", "roblox-studio-auth:1+ticket:abc"]));

        assert_eq!(parsed.roblox_launch_mode, LaunchMode::StudioAuth);
        assert!(parsed.roblox_launch_args.starts_with("roblox-studio-auth:"));
    }

    #[test]
    fn parses_tray_host_payload() {
        let parsed = ParsedLaunchSettings::parse(&as_args(&["-trayhost", "abc123"]));
        assert!(parsed.tray_host_flag.active);
        assert_eq!(parsed.tray_host_flag.data.as_deref(), Some("abc123"));
    }
}
