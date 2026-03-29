/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use regex::Regex;

use crate::activity_data::ActivityData;
use crate::enums::ServerType;
use crate::roblox_api::RpcMessage;

const RUSTSTRAP_GAME_MESSAGE_ENTRY: &str = "[FLog::Output] [RuststrapRPC]";
const BLOXSTRAP_GAME_MESSAGE_ENTRY: &str = "[FLog::Output] [BloxstrapRPC]";
const GAME_JOINING_ENTRY: &str = "[FLog::Output] ! Joining game";
const GAME_TELEPORTING_ENTRY: &str = "[FLog::GameJoinUtil] GameJoinUtil::initiateTeleportToPlace";
const GAME_JOINING_PRIVATE_SERVER: &str =
    "[FLog::GameJoinUtil] GameJoinUtil::joinGamePostPrivateServer";
const GAME_JOINING_RESERVED_SERVER: &str =
    "[FLog::GameJoinUtil] GameJoinUtil::initiateTeleportToReservedServer";
const GAME_JOINING_UNIVERSE_ENTRY: &str = "[FLog::GameJoinLoadTime] Report game_join_loadtime:";
const GAME_JOINING_UDMUX_ENTRY: &str = "[FLog::Network] UDMUX Address = ";
const GAME_JOINED_ENTRY: &str = "[FLog::Network] serverId:";
const GAME_DISCONNECTED_ENTRY: &str = "[FLog::Network] Time to disconnect replication data:";
const GAME_DISCONNECT_REASON_ENTRY: &str = "[FLog::Network] Sending disconnect with reason:";
const GAME_LEAVING_ENTRY: &str = "[FLog::SingleSurfaceApp] leaveUGCGameInternal";
const GAME_SERVER_UPTIME_ENTRY: &str = "[FLog::Output] Server Prefix: ";
const PLAYER_LOG_RECENCY_WINDOW_SECS: u64 = 15;
const PLAYER_LOG_BIND_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityState {
    Idle,
    Joining,
    InGame,
    InHome,
}

#[derive(Debug, Clone)]
pub enum ActivityEvent {
    LogEntry(String),
    GameJoin,
    GameLeave,
    HomeEntered,
    AppClose,
    LogOpen(String),
    RpcMessage(RpcMessage),
    ServerUptime(String),
    DisconnectReason(u32),
}

#[derive(Debug)]
pub struct ActivityWatcherState {
    pub in_game: bool,
    pub state: ActivityState,
    pub data: ActivityData,
    pub history: VecDeque<ActivityData>,
    pub log_location: String,
    pub events: Vec<ActivityEvent>,
    pub last_disconnect_reason: Option<u32>,
    teleport_marker: bool,
    reserved_teleport_marker: bool,
    log_entries_read: u64,
}

impl Default for ActivityWatcherState {
    fn default() -> Self {
        Self {
            in_game: false,
            state: ActivityState::Idle,
            data: ActivityData::default(),
            history: VecDeque::with_capacity(50),
            log_location: String::new(),
            events: Vec::new(),
            last_disconnect_reason: None,
            teleport_marker: false,
            reserved_teleport_marker: false,
            log_entries_read: 0,
        }
    }
}

pub struct ActivityWatcher {
    pub state: Arc<Mutex<ActivityWatcherState>>,
    stop_flag: Arc<Mutex<bool>>,
}

impl ActivityWatcher {
    pub fn new(log_file: Option<String>) -> Self {
        let mut state = ActivityWatcherState::default();
        if let Some(log) = log_file {
            state.log_location = log;
        }
        Self {
            state: Arc::new(Mutex::new(state)),
            stop_flag: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) {
        let log_location = {
            let state = self.state.lock().unwrap();
            state.log_location.clone()
        };

        let log_path = if log_location.is_empty() {
            let mut selected = wait_for_recent_player_log(
                Duration::from_secs(PLAYER_LOG_RECENCY_WINDOW_SECS),
                Duration::from_secs(PLAYER_LOG_BIND_TIMEOUT_SECS),
                Duration::from_millis(500),
            );
            if selected.is_none() {
                selected = newest_player_log_file();
            }

            let path = match selected {
                Some(path) => path,
                None => loop {
                    if self.should_stop() {
                        return;
                    }
                    if let Some(path) = newest_player_log_file() {
                        let path_str = path.to_string_lossy().to_string();
                        let mut state = self.state.lock().unwrap();
                        state.log_location = path_str.clone();
                        state.events.push(ActivityEvent::LogOpen(path_str));
                        break path;
                    }
                    thread::sleep(Duration::from_secs(1));
                },
            };

            let path_str = path.to_string_lossy().to_string();
            let mut state = self.state.lock().unwrap();
            state.log_location = path_str.clone();
            state.events.push(ActivityEvent::LogOpen(path_str));
            path
        } else {
            let mut state = self.state.lock().unwrap();
            state
                .events
                .push(ActivityEvent::LogOpen(log_location.clone()));
            PathBuf::from(&log_location)
        };

        let mut reader = loop {
            if self.should_stop() {
                return;
            }

            match File::open(&log_path) {
                Ok(file) => break BufReader::new(file),
                Err(_) => thread::sleep(Duration::from_secs(1)),
            }
        };

        loop {
            if self.should_stop() {
                break;
            }

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    thread::sleep(Duration::from_secs(1));
                }
                Ok(_) => {
                    let trimmed = line.trim_end().to_string();
                    if !trimmed.is_empty() {
                        self.read_log_entry(&trimmed);
                    }
                }
                Err(_) => {
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }
    }

    pub fn stop(&self) {
        let mut stop = self.stop_flag.lock().unwrap();
        *stop = true;
    }

    fn should_stop(&self) -> bool {
        *self.stop_flag.lock().unwrap()
    }

    pub fn drain_events(&self) -> Vec<ActivityEvent> {
        let mut state = self.state.lock().unwrap();
        std::mem::take(&mut state.events)
    }

    fn read_log_entry(&self, entry: &str) {
        let mut state = self.state.lock().unwrap();
        state.log_entries_read += 1;
        state
            .events
            .push(ActivityEvent::LogEntry(entry.to_string()));

        let log_message = match entry.find(' ') {
            Some(idx) => &entry[idx + 1..],
            None => return,
        };

        if log_message.starts_with(GAME_DISCONNECT_REASON_ENTRY) {
            if let Some(reason) = parse_disconnect_reason(log_message) {
                state.last_disconnect_reason = Some(reason);
                state.events.push(ActivityEvent::DisconnectReason(reason));
            }
        }

        if log_message.starts_with(GAME_LEAVING_ENTRY) {
            transition_to_home(&mut state);
            return;
        }

        if !state.in_game && state.data.place_id == 0 {
            if log_message.starts_with(GAME_JOINING_PRIVATE_SERVER) {
                state.state = ActivityState::Joining;
                state.data.server_type = ServerType::Private;
                if let Some(access_code) = parse_private_access_code(log_message) {
                    state.data.access_code = access_code;
                }
            } else if log_message.starts_with(GAME_JOINING_ENTRY) {
                if let Some((job_id, place_id, machine_address)) = parse_joining_line(log_message) {
                    state.in_game = false;
                    state.state = ActivityState::Joining;
                    state.data.place_id = place_id;
                    state.data.job_id = job_id;
                    state.data.machine_address = machine_address;

                    if state.teleport_marker {
                        state.data.is_teleport = true;
                        state.teleport_marker = false;
                    }
                    if state.reserved_teleport_marker {
                        state.data.server_type = ServerType::Reserved;
                        state.reserved_teleport_marker = false;
                    }
                }
            }
        } else if !state.in_game && state.data.place_id != 0 {
            if log_message.starts_with(GAME_DISCONNECTED_ENTRY) {
                transition_to_home(&mut state);
            } else if log_message.starts_with(GAME_JOINING_UNIVERSE_ENTRY) {
                if let Some((universe_id, user_id)) = parse_joining_universe(log_message) {
                    state.data.universe_id = universe_id;
                    state.data.user_id = user_id;
                }
            } else if log_message.starts_with(GAME_JOINING_UDMUX_ENTRY) {
                if let Some((udmux_ip, rcc_ip)) = parse_udmux(log_message) {
                    if same_server_address(&rcc_ip, &state.data.machine_address) {
                        state.data.machine_address = udmux_ip;
                    }
                }
            } else if log_message.starts_with(GAME_JOINED_ENTRY) {
                if let Some(server_ip) = parse_joined_server_id(log_message) {
                    if state.data.machine_address.is_empty()
                        || same_server_address(&server_ip, &state.data.machine_address)
                    {
                        state.in_game = true;
                        state.state = ActivityState::InGame;
                        state.data.time_joined = Some(chrono_now_iso());
                        state.events.push(ActivityEvent::GameJoin);
                    }
                }
            }
        } else if state.in_game && state.data.place_id != 0 {
            if log_message.starts_with(GAME_DISCONNECTED_ENTRY) {
                state.data.time_left = Some(chrono_now_iso());
                let finished = state.data.clone();
                push_history(&mut state.history, finished);
                state.in_game = false;
                state.data.reset();
                state.state = ActivityState::InHome;
                state.events.push(ActivityEvent::GameLeave);
                state.events.push(ActivityEvent::HomeEntered);
            } else if log_message.starts_with(GAME_TELEPORTING_ENTRY) {
                state.teleport_marker = true;
            } else if log_message.starts_with(GAME_JOINING_RESERVED_SERVER) {
                state.teleport_marker = true;
                state.reserved_teleport_marker = true;
            } else if log_message.starts_with(RUSTSTRAP_GAME_MESSAGE_ENTRY)
                || log_message.starts_with(BLOXSTRAP_GAME_MESSAGE_ENTRY)
            {
                if let Some(message) = parse_rpc_message(log_message, &mut state.data) {
                    state.events.push(ActivityEvent::RpcMessage(message));
                }
            } else if entry.contains(GAME_SERVER_UPTIME_ENTRY) {
                if let Some(start_time) = parse_server_uptime(entry) {
                    state.data.start_time = Some(start_time.clone());
                    state.events.push(ActivityEvent::ServerUptime(start_time));
                }
            }
        }
    }
}

fn transition_to_home(state: &mut ActivityWatcherState) {
    let was_in_game = state.in_game;
    let had_join_context = state.data.place_id != 0;

    if was_in_game {
        state.data.time_left = Some(chrono_now_iso());
        push_history(&mut state.history, state.data.clone());
        state.events.push(ActivityEvent::GameLeave);
    }

    if had_join_context {
        state.data.reset();
    }

    state.in_game = false;

    if state.state != ActivityState::InHome {
        state.events.push(ActivityEvent::HomeEntered);
    }
    state.state = ActivityState::InHome;
}

fn push_history(history: &mut VecDeque<ActivityData>, data: ActivityData) {
    history.push_front(data);
    if history.len() > 50 {
        history.pop_back();
    }
}

fn parse_private_access_code(log_message: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)"accesscode":"([0-9a-f\-]{36})""#).ok()?;
    let caps = re.captures(log_message)?;
    Some(caps.get(1)?.as_str().to_string())
}

fn parse_joining_line(log_message: &str) -> Option<(String, i64, String)> {
    let re = Regex::new(r"! Joining game '([0-9a-fA-F\-]{36})' place ([0-9]+) at ([^ ]+)").ok()?;
    let caps = re.captures(log_message)?;
    let job_id = caps.get(1)?.as_str().to_string();
    let place_id = caps.get(2)?.as_str().parse().ok()?;
    let machine = caps.get(3)?.as_str().to_string();
    Some((job_id, place_id, machine))
}

fn parse_joining_universe(log_message: &str) -> Option<(i64, i64)> {
    let re = Regex::new(r"(?i)universeid:([0-9]+).*userid:([0-9]+)").ok()?;
    let caps = re.captures(log_message)?;
    let universe_id = caps.get(1)?.as_str().parse().ok()?;
    let user_id = caps.get(2)?.as_str().parse().ok()?;
    Some((universe_id, user_id))
}

fn parse_udmux(log_message: &str) -> Option<(String, String)> {
    let re = Regex::new(
        r"UDMUX Address = ([0-9A-Fa-f\.\:]+), Port = [0-9]+ \| RCC Server Address = ([0-9A-Fa-f\.\:]+), Port = [0-9]+",
    )
    .ok()?;
    let caps = re.captures(log_message)?;
    let udmux_ip = caps.get(1)?.as_str().to_string();
    let rcc_ip = caps.get(2)?.as_str().to_string();
    Some((udmux_ip, rcc_ip))
}

fn parse_joined_server_id(log_message: &str) -> Option<String> {
    let re = Regex::new(r"(?i)serverId:\s*([^| ]+)\|[0-9]+").ok()?;
    let caps = re.captures(log_message)?;
    Some(caps.get(1)?.as_str().to_string())
}

fn parse_rpc_message(log_message: &str, activity_data: &mut ActivityData) -> Option<RpcMessage> {
    let re = Regex::new(r"\[(?:RuststrapRPC|BloxstrapRPC)\] (.*)").ok()?;
    let caps = re.captures(log_message)?;
    let message_plain = caps.get(1)?.as_str();
    let message = serde_json::from_str::<RpcMessage>(message_plain).ok()?;

    if message.command == "SetLaunchData" {
        if let Some(data) = &message.data {
            if let Some(text) = data.as_str() {
                if text.len() <= 200 {
                    activity_data.rpc_launch_data = text.to_string();
                }
            }
        }
    }

    if message.command.is_empty() {
        None
    } else {
        Some(message)
    }
}

fn parse_server_uptime(entry: &str) -> Option<String> {
    let re = Regex::new(r"Server Prefix:.+_(\d{8}T\d{6}Z)_RCC_[0-9a-z]+").ok()?;
    let caps = re.captures(entry)?;
    Some(caps.get(1)?.as_str().to_string())
}

fn parse_disconnect_reason(log_message: &str) -> Option<u32> {
    let re = Regex::new(r"Sending disconnect with reason: (\d+)").ok()?;
    let caps = re.captures(log_message)?;
    caps.get(1)?.as_str().parse::<u32>().ok()
}

fn same_server_address(left: &str, right: &str) -> bool {
    let left = left
        .trim()
        .trim_matches(|value| value == '[' || value == ']')
        .to_ascii_lowercase();
    let right = right
        .trim()
        .trim_matches(|value| value == '[' || value == ']')
        .to_ascii_lowercase();
    !left.is_empty() && !right.is_empty() && left == right
}

pub fn newest_player_log_file() -> Option<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let log_dir = Path::new(&local_app_data).join("Roblox").join("logs");

    if !log_dir.exists() {
        return None;
    }

    let mut newest: Option<(SystemTime, PathBuf)> = None;

    for entry in std::fs::read_dir(&log_dir).ok()?.filter_map(|e| e.ok()) {
        if !entry.file_name().to_string_lossy().contains("Player") {
            continue;
        }

        let timestamp = entry
            .metadata()
            .ok()
            .and_then(|m| m.created().ok().or_else(|| m.modified().ok()));

        let Some(timestamp) = timestamp else {
            continue;
        };

        match &newest {
            Some((latest, _)) if timestamp <= *latest => {}
            _ => newest = Some((timestamp, entry.path())),
        }
    }

    newest.map(|(_, path)| path)
}

pub fn find_recent_player_log_file(max_age: Duration) -> Option<PathBuf> {
    let now = SystemTime::now();
    let path = newest_player_log_file()?;
    let timestamp = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.created().ok().or_else(|| m.modified().ok()))?;
    let age = now
        .duration_since(timestamp)
        .unwrap_or_else(|_| Duration::from_secs(0));
    if age <= max_age {
        Some(path)
    } else {
        None
    }
}

pub fn wait_for_recent_player_log(
    max_age: Duration,
    timeout: Duration,
    poll_interval: Duration,
) -> Option<PathBuf> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(path) = find_recent_player_log_file(max_age) {
            return Some(path);
        }

        if Instant::now() >= deadline {
            return None;
        }

        thread::sleep(poll_interval);
    }
}

fn chrono_now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_game_joining_entry() {
        let watcher = ActivityWatcher::new(None);
        let entry = "2024-01-01T00:00:00Z [FLog::Output] ! Joining game 'abcd1234-5678-90ab-cdef-1234567890ab' place 123456 at 192.168.1.1";
        watcher.read_log_entry(entry);
        let state = watcher.state.lock().unwrap();
        assert_eq!(state.state, ActivityState::Joining);
        assert_eq!(state.data.place_id, 123456);
        assert_eq!(state.data.job_id, "abcd1234-5678-90ab-cdef-1234567890ab");
        assert_eq!(state.data.machine_address, "192.168.1.1");
    }

    #[test]
    fn parse_game_leave_enters_home_state() {
        let watcher = ActivityWatcher::new(None);
        let join_entry = "2024-01-01T00:00:00Z [FLog::Output] ! Joining game 'abcd1234-5678-90ab-cdef-1234567890ab' place 123 at 1.2.3.4";
        let joined_entry = "2024-01-01T00:00:01Z [FLog::Network] serverId: 1.2.3.4|1234";
        let leave_entry = "2024-01-01T00:00:02Z [FLog::SingleSurfaceApp] leaveUGCGameInternal";

        watcher.read_log_entry(join_entry);
        watcher.read_log_entry(joined_entry);
        watcher.read_log_entry(leave_entry);

        let events = watcher.drain_events();
        assert!(events
            .iter()
            .any(|event| matches!(event, ActivityEvent::GameLeave)));
        assert!(events
            .iter()
            .any(|event| matches!(event, ActivityEvent::HomeEntered)));
        assert!(!events
            .iter()
            .any(|event| matches!(event, ActivityEvent::AppClose)));

        let state = watcher.state.lock().unwrap();
        assert!(!state.in_game);
        assert_eq!(state.state, ActivityState::InHome);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn parse_ruststrap_rpc_message_tag() {
        let watcher = ActivityWatcher::new(None);
        let join_entry = "2024-01-01T00:00:00Z [FLog::Output] ! Joining game 'abcd1234-5678-90ab-cdef-1234567890ab' place 123 at 1.2.3.4";
        let joined_entry = "2024-01-01T00:00:01Z [FLog::Network] serverId: 1.2.3.4|1234";
        let rpc_entry = "2024-01-01T00:00:02Z [FLog::Output] [RuststrapRPC] {\"command\":\"SetRichPresence\",\"data\":{\"details\":\"Test\"}}";

        watcher.read_log_entry(join_entry);
        watcher.read_log_entry(joined_entry);
        watcher.read_log_entry(rpc_entry);

        let events = watcher.drain_events();
        assert!(events.iter().any(|event| {
            matches!(
                event,
                ActivityEvent::RpcMessage(msg) if msg.command == "SetRichPresence"
            )
        }));
    }

    #[test]
    fn parse_bloxstrap_rpc_message_tag() {
        let watcher = ActivityWatcher::new(None);
        let join_entry = "2024-01-01T00:00:00Z [FLog::Output] ! Joining game 'abcd1234-5678-90ab-cdef-1234567890ab' place 123 at 1.2.3.4";
        let joined_entry = "2024-01-01T00:00:01Z [FLog::Network] serverId: 1.2.3.4|1234";
        let rpc_entry = "2024-01-01T00:00:02Z [FLog::Output] [BloxstrapRPC] {\"command\":\"SetRichPresence\",\"data\":{\"details\":\"Test\"}}";

        watcher.read_log_entry(join_entry);
        watcher.read_log_entry(joined_entry);
        watcher.read_log_entry(rpc_entry);

        let events = watcher.drain_events();
        assert!(events.iter().any(|event| {
            matches!(
                event,
                ActivityEvent::RpcMessage(msg) if msg.command == "SetRichPresence"
            )
        }));
    }

    #[test]
    fn parse_disconnect_reason_event() {
        let watcher = ActivityWatcher::new(None);
        watcher.read_log_entry(
            "2024-01-01T00:00:00Z [FLog::Network] Sending disconnect with reason: 277",
        );
        let events = watcher.drain_events();
        assert!(events.iter().any(|event| {
            matches!(event, ActivityEvent::DisconnectReason(code) if *code == 277)
        }));
    }
}
