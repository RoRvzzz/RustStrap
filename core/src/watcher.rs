use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::activity_data::ActivityData;
use crate::activity_watcher::{ActivityEvent, ActivityWatcher};
use crate::discord_rpc::{
    fetch_thumbnail_url, fetch_universe_details, fetch_user_display, query_server_location,
    DiscordRichPresence, RpcDisplaySettings,
};
use crate::errors::{DomainError, Result};
use crate::launch_handler::open_url;
use crate::process_utils::configure_hidden;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherData {
    #[serde(rename = "ProcessId")]
    pub process_id: u32,
    #[serde(rename = "LogFile")]
    pub log_file: String,
    #[serde(rename = "AutoclosePids", default)]
    pub autoclose_pids: Vec<u32>,
    #[serde(rename = "Handle", default)]
    pub handle: i64,
    #[serde(rename = "LaunchMode", default)]
    pub launch_mode: String,
    #[serde(rename = "UseDiscordRichPresence", default)]
    pub use_discord_rich_presence: bool,
    #[serde(rename = "HideRPCButtons", default)]
    pub hide_rpc_buttons: bool,
    #[serde(rename = "ShowAccountOnRichPresence", default)]
    pub show_account_on_rich_presence: bool,
    #[serde(rename = "EnableCustomStatusDisplay", default = "default_true")]
    pub enable_custom_status_display: bool,
    #[serde(rename = "ShowUsingRuststrapRPC", default = "default_true")]
    pub show_using_ruststrap_rpc: bool,
    #[serde(rename = "ShowServerDetails", default)]
    pub show_server_details: bool,
    #[serde(rename = "ShowServerUptime", default)]
    pub show_server_uptime: bool,
    #[serde(rename = "PlaytimeCounter", default = "default_true")]
    pub playtime_counter: bool,
    #[serde(rename = "AutoRejoin", default)]
    pub auto_rejoin: bool,
    #[serde(rename = "UseDisableAppPatch", default)]
    pub use_disable_app_patch: bool,
}

fn default_true() -> bool {
    true
}

pub struct Watcher {
    data: WatcherData,
    running: bool,
}

impl Watcher {
    pub fn new(data: WatcherData) -> Self {
        Self {
            data,
            running: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.running = true;

        let rpc_settings = RpcDisplaySettings {
            hide_rpc_buttons: self.data.hide_rpc_buttons,
            show_account_on_rich_presence: self.data.show_account_on_rich_presence,
            enable_custom_status_display: self.data.enable_custom_status_display,
            show_using_ruststrap_rpc: self.data.show_using_ruststrap_rpc,
            show_server_details: self.data.show_server_details,
            show_server_uptime: self.data.show_server_uptime,
            playtime_counter: self.data.playtime_counter,
        };

        let rpc = if self.data.use_discord_rich_presence {
            let client = DiscordRichPresence::new();
            client.connect();
            client.set_home_presence(&rpc_settings);
            Some(client)
        } else {
            None
        };

        let log_file = if self.data.log_file.is_empty() {
            None
        } else {
            Some(self.data.log_file.clone())
        };
        let activity_watcher = ActivityWatcher::new(log_file);
        let watcher_state = activity_watcher.state.clone();

        thread::spawn(move || {
            activity_watcher.start();
        });

        let mut disconnect_reason: Option<u32> = None;
        let mut active_activity: Option<ActivityData> = None;

        loop {
            thread::sleep(Duration::from_secs(1));

            if self.data.process_id != 0 && !self.is_process_alive(self.data.process_id) {
                break;
            }

            let events = {
                let mut state = watcher_state.lock().unwrap();
                std::mem::take(&mut state.events)
            };

            let mut should_exit = false;
            for event in events {
                match event {
                    ActivityEvent::GameJoin => {
                        disconnect_reason = None;
                        let data = {
                            let state = watcher_state.lock().unwrap();
                            state.data.clone()
                        };
                        active_activity = Some(data.clone());

                        if let Some(rpc_client) = rpc.clone() {
                            let rpc_settings = rpc_settings.clone();
                            thread::spawn(move || {
                                if data.universe_id == 0 {
                                    return;
                                }

                                if let Ok(Some(details)) = fetch_universe_details(data.universe_id)
                                {
                                    let icon_url = fetch_thumbnail_url(
                                        data.universe_id as u64,
                                        "UniverseThumbnail",
                                        "512x512",
                                    )
                                    .unwrap_or_default()
                                    .unwrap_or_else(|| "roblox".to_string());

                                    let user_display = if rpc_settings.show_account_on_rich_presence
                                    {
                                        fetch_user_display(data.user_id).ok().flatten()
                                    } else {
                                        None
                                    };

                                    let server_location = if rpc_settings.show_server_details
                                        && data.machine_address_valid()
                                    {
                                        query_server_location(&data.machine_address).ok().flatten()
                                    } else {
                                        None
                                    };

                                    let server_uptime = if rpc_settings.show_server_uptime {
                                        data.start_time.as_deref().and_then(format_server_uptime)
                                    } else {
                                        None
                                    };

                                    rpc_client.set_current_game(
                                        &data,
                                        &details.name,
                                        &details.creator.name,
                                        details.creator.has_verified_badge,
                                        &icon_url,
                                        &rpc_settings,
                                        user_display.as_ref(),
                                        server_location.as_deref(),
                                        server_uptime.as_deref(),
                                    );
                                }
                            });
                        }
                    }
                    ActivityEvent::DisconnectReason(code) => {
                        disconnect_reason = Some(code);
                    }
                    ActivityEvent::ServerUptime(start_tag) => {
                        if let Some(rpc_client) = rpc.as_ref() {
                            if let Some(activity) = active_activity.as_ref() {
                                if activity.universe_id != 0 {
                                    if let Ok(Some(details)) =
                                        fetch_universe_details(activity.universe_id)
                                    {
                                        let icon_url = fetch_thumbnail_url(
                                            activity.universe_id as u64,
                                            "UniverseThumbnail",
                                            "512x512",
                                        )
                                        .unwrap_or_default()
                                        .unwrap_or_else(|| "roblox".to_string());

                                        let user_display =
                                            if rpc_settings.show_account_on_rich_presence {
                                                fetch_user_display(activity.user_id).ok().flatten()
                                            } else {
                                                None
                                            };

                                        let server_location = if rpc_settings.show_server_details
                                            && activity.machine_address_valid()
                                        {
                                            query_server_location(&activity.machine_address)
                                                .ok()
                                                .flatten()
                                        } else {
                                            None
                                        };

                                        let server_uptime = if rpc_settings.show_server_uptime {
                                            format_server_uptime(&start_tag)
                                        } else {
                                            None
                                        };

                                        rpc_client.set_current_game(
                                            activity,
                                            &details.name,
                                            &details.creator.name,
                                            details.creator.has_verified_badge,
                                            &icon_url,
                                            &rpc_settings,
                                            user_display.as_ref(),
                                            server_location.as_deref(),
                                            server_uptime.as_deref(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    ActivityEvent::GameLeave => {
                        if self.data.auto_rejoin
                            && disconnect_reason.map(should_auto_rejoin).unwrap_or(false)
                        {
                            if let Some(activity) = active_activity.as_ref() {
                                let _ = self.try_rejoin(activity);
                            }
                        }

                        disconnect_reason = None;
                        active_activity = None;

                        if let Some(rpc_client) = rpc.as_ref() {
                            rpc_client.set_home_presence(&rpc_settings);
                        }
                    }
                    ActivityEvent::HomeEntered => {
                        disconnect_reason = None;
                        active_activity = None;

                        if self.data.use_disable_app_patch && self.data.process_id != 0 {
                            self.close_process(self.data.process_id, false);
                            should_exit = true;
                            break;
                        } else if let Some(rpc_client) = rpc.as_ref() {
                            rpc_client.set_home_presence(&rpc_settings);
                        }
                    }
                    ActivityEvent::RpcMessage(message) => {
                        if let Some(rpc_client) = rpc.as_ref() {
                            rpc_client.process_rpc_message(&message);
                        }
                    }
                    ActivityEvent::AppClose => {
                        should_exit = true;
                        break;
                    }
                    ActivityEvent::LogEntry(_) | ActivityEvent::LogOpen(_) => {}
                }
            }

            if should_exit {
                break;
            }
        }

        if let Some(rpc_client) = rpc.as_ref() {
            rpc_client.clear_presence();
        }

        for pid in &self.data.autoclose_pids {
            self.close_process(*pid, false);
        }

        self.running = false;
        Ok(())
    }

    pub fn kill_roblox_process(&self) {
        self.close_process(self.data.process_id, true);
    }

    pub fn close_process(&self, pid: u32, force: bool) {
        if force {
            let mut command = Command::new("taskkill");
            configure_hidden(&mut command);
            let _ = command.args(["/F", "/PID", &pid.to_string()]).output();
        } else {
            let mut command = Command::new("taskkill");
            configure_hidden(&mut command);
            let _ = command.args(["/PID", &pid.to_string()]).output();
        }
    }

    fn try_rejoin(&self, activity: &ActivityData) -> Result<()> {
        if activity.place_id == 0 {
            return Ok(());
        }

        let deeplink = activity.get_invite_deeplink(true);
        open_url(&deeplink)
    }

    #[cfg(windows)]
    fn is_process_alive(&self, pid: u32) -> bool {
        extern "system" {
            fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
            fn GetExitCodeProcess(process: isize, exit_code: *mut u32) -> i32;
            fn CloseHandle(handle: isize) -> i32;
        }

        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const STILL_ACTIVE: u32 = 259;

        if pid == 0 {
            return false;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }

            let mut exit_code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut exit_code as *mut u32) != 0;
            let _ = CloseHandle(handle);
            ok && exit_code == STILL_ACTIVE
        }
    }

    #[cfg(not(windows))]
    fn is_process_alive(&self, _pid: u32) -> bool {
        false
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn process_id(&self) -> u32 {
        self.data.process_id
    }

    pub fn log_file(&self) -> &str {
        &self.data.log_file
    }
}

fn should_auto_rejoin(reason: u32) -> bool {
    matches!(reason, 1 | 277)
}

fn format_server_uptime(start_time_tag: &str) -> Option<String> {
    let started_at = NaiveDateTime::parse_from_str(start_time_tag, "%Y%m%dT%H%M%SZ").ok()?;
    let started_at = DateTime::<Utc>::from_naive_utc_and_offset(started_at, Utc);
    let elapsed = Utc::now() - started_at;

    if elapsed.num_seconds() < 0 {
        return None;
    }

    Some(format_duration(elapsed.num_seconds() as u64))
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

pub fn encode_watcher_data(data: &WatcherData) -> Result<String> {
    let json = serde_json::to_string(data)
        .map_err(|e| DomainError::Serialization(format!("watcher data serialize failed: {e}")))?;
    Ok(base64_encode(json.as_bytes()))
}

pub fn decode_watcher_data(encoded: &str) -> Result<WatcherData> {
    let decoded = base64_decode_simple(encoded)?;
    let json = String::from_utf8(decoded)
        .map_err(|e| DomainError::Serialization(format!("watcher data decode failed: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| DomainError::Serialization(format!("watcher data parse failed: {e}")))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode_simple(input: &str) -> Result<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();

    for chunk in bytes.chunks(4) {
        let mut buf = [0u8; 4];
        let mut count = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                buf[i] = 0;
            } else {
                buf[i] = TABLE.iter().position(|&c| c == byte).unwrap_or(0) as u8;
                count = i + 1;
            }
        }
        if count == 0 {
            break;
        }
        result.push((buf[0] << 2) | (buf[1] >> 4));
        if count > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if count > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_data_round_trip() {
        let data = WatcherData {
            process_id: 1234,
            log_file: "test.log".to_string(),
            autoclose_pids: vec![5678],
            handle: 0,
            launch_mode: "player".to_string(),
            use_discord_rich_presence: true,
            hide_rpc_buttons: false,
            show_account_on_rich_presence: true,
            enable_custom_status_display: true,
            show_using_ruststrap_rpc: true,
            show_server_details: true,
            show_server_uptime: false,
            playtime_counter: true,
            auto_rejoin: true,
            use_disable_app_patch: false,
        };

        let encoded = encode_watcher_data(&data).unwrap();
        let decoded = decode_watcher_data(&encoded).unwrap();

        assert_eq!(decoded.process_id, 1234);
        assert_eq!(decoded.log_file, "test.log");
        assert_eq!(decoded.autoclose_pids, vec![5678]);
        assert!(decoded.use_discord_rich_presence);
        assert!(decoded.show_account_on_rich_presence);
        assert!(decoded.auto_rejoin);
    }

    #[test]
    fn uptime_formatter_works() {
        let text = format_server_uptime("20260101T000000Z");
        assert!(text.is_some());
    }
}
