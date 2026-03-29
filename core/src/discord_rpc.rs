use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use discord_presence::Client as DiscordClient;
use crate::activity_data::ActivityData;
use crate::enums::ServerType;
use crate::errors::{DomainError, Result};
use crate::roblox_api::{
    GameDetailData, GameDetailResponse, GetUserResponse, RpcMessage, RpcRichPresence,
    ThumbnailBatchResponse, ThumbnailCacheEntry,
};

/// I hate this shit bro

pub const DISCORD_APP_ID: &str = "1486934007370354848";
const RUSTSTRAP_IMAGE_SOURCE: &str =
    "https://raw.githubusercontent.com/RoRvzzz/RustStrap/main/interface/public/icon.png";
const RUSTSTRAP_HOME_LARGE_IMAGE_KEY: &str = RUSTSTRAP_IMAGE_SOURCE;
const RUSTSTRAP_HOME_SMALL_IMAGE_KEY: &str = RUSTSTRAP_IMAGE_SOURCE;
const RUSTSTRAP_GAME_FALLBACK_IMAGE_KEY: &str = RUSTSTRAP_IMAGE_SOURCE;
const RUSTSTRAP_USER_FALLBACK_IMAGE_KEY: &str = RUSTSTRAP_IMAGE_SOURCE;
const THUMBNAIL_RETRY_ATTEMPTS: usize = 3;
const THUMBNAIL_RETRY_DELAY_MS: u64 = 350;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RpcDisplaySettings {
    pub hide_rpc_buttons: bool,
    pub show_account_on_rich_presence: bool,
    pub enable_custom_status_display: bool,
    pub show_using_ruststrap_rpc: bool,
    pub show_server_details: bool,
    pub show_server_uptime: bool,
    pub playtime_counter: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RobloxUserDisplay {
    pub headshot_url: String,
    pub display_name: String,
    pub username: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PresenceData {
    pub details: String,
    pub state: String,
    pub large_image_key: String,
    pub large_image_text: String,
    pub small_image_key: String,
    pub small_image_text: String,
    pub timestamp_start: Option<i64>,
    pub timestamp_end: Option<i64>,
    pub buttons: Vec<PresenceButton>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PresenceButton {
    pub label: String,
    pub url: String,
}

#[derive(Clone)]
pub struct DiscordRichPresence {
    presence: Arc<Mutex<Option<PresenceData>>>,
    original_presence: Arc<Mutex<Option<PresenceData>>>,
    thumbnail_cache: Arc<Mutex<Vec<ThumbnailCacheEntry>>>,
    visible: Arc<Mutex<bool>>,
    client: Arc<Mutex<Option<DiscordClient>>>,
}

impl DiscordRichPresence {
    pub fn new() -> Self {
        Self {
            presence: Arc::new(Mutex::new(None)),
            original_presence: Arc::new(Mutex::new(None)),
            thumbnail_cache: Arc::new(Mutex::new(Vec::with_capacity(20))),
            visible: Arc::new(Mutex::new(true)),
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub fn connect(&self) {
        let presence = self.presence.clone();
        let visible = self.visible.clone();
        let client_state = self.client.clone();

        thread::spawn(move || {
            let app_id = match DISCORD_APP_ID.parse() {
                Ok(value) => value,
                Err(error) => {
                    log::warn!("discord app id parse failed: {error}");
                    return;
                }
            };
            let mut client = DiscordClient::new(app_id);

            client.on_ready(|_ctx| {}).persist();

            client.start();
            *client_state.lock().unwrap() = Some(client);
            let mut last_had_timestamps = false;

            loop {
                thread::sleep(Duration::from_secs(5));

                let pres_data = {
                    let is_visible = *visible.lock().unwrap();
                    if !is_visible {
                        None
                    } else {
                        presence.lock().unwrap().clone()
                    }
                };

                let mut client_lock = client_state.lock().unwrap();
                if let Some(client) = client_lock.as_mut() {
                    match pres_data {
                        Some(data) => {
                            let has_timestamps =
                                data.timestamp_start.is_some() || data.timestamp_end.is_some();
                            if last_had_timestamps && !has_timestamps {
                                let _ = client.clear_activity();
                            }
                            last_had_timestamps = has_timestamps;

                            let _ = client.set_activity(|activity| {
                                let large_image_key = normalize_discord_image_key(
                                    &data.large_image_key,
                                    RUSTSTRAP_GAME_FALLBACK_IMAGE_KEY,
                                );
                                let small_image_key = normalize_discord_image_key(
                                    &data.small_image_key,
                                    RUSTSTRAP_USER_FALLBACK_IMAGE_KEY,
                                );

                                let mut activity = activity
                                    .details(&data.details)
                                    .state(&data.state)
                                    .assets(|assets| {
                                        let mut assets = assets
                                            .large_image(&large_image_key)
                                            .large_text(&data.large_image_text);
                                        if !small_image_key.is_empty() {
                                            assets = assets
                                                .small_image(&small_image_key)
                                                .small_text(&data.small_image_text);
                                        }
                                        assets
                                    });

                                if has_timestamps {
                                    activity = activity.timestamps(|timestamps| {
                                        let mut timestamps = timestamps;
                                        if let Some(start) = data.timestamp_start {
                                            timestamps = timestamps.start(start as u64);
                                        }
                                        if let Some(end) = data.timestamp_end {
                                            timestamps = timestamps.end(end as u64);
                                        }
                                        timestamps
                                    });
                                }

                                for button in &data.buttons {
                                    activity = activity.append_buttons(|btn| {
                                        btn.label(&button.label).url(&button.url)
                                    });
                                }

                                activity
                            });
                        }
                        None => {
                            last_had_timestamps = false;
                            let _ = client.clear_activity();
                        }
                    }
                }
            }
        });
    }

    pub fn set_current_game(
        &self,
        activity: &ActivityData,
        universe_name: &str,
        creator_name: &str,
        has_verified_badge: bool,
        icon_url: &str,
        settings: &RpcDisplaySettings,
        user_display: Option<&RobloxUserDisplay>,
        server_location: Option<&str>,
        server_uptime: Option<&str>,
    ) {
        let verified = if has_verified_badge { " ☑️" } else { "" };
        let mut status = match activity.server_type {
            ServerType::Private => "In a private server".to_string(),
            ServerType::Reserved => "In a reserved server".to_string(),
            ServerType::Public => format!("by {creator_name}{verified}"),
        };

        if settings.show_server_details {
            if let Some(location) = server_location.filter(|value| !value.trim().is_empty()) {
                status = format!("{status} | {location}");
            }
        }

        if settings.show_server_uptime {
            if let Some(uptime) = server_uptime.filter(|value| !value.trim().is_empty()) {
                status = format!("{status} | up {uptime}");
            }
        }

        if settings.show_using_ruststrap_rpc {
            status = format!("{status} • Ruststrap");
        }

        let mut display_name = universe_name.to_string();
        if display_name.len() < 2 {
            display_name = format!("{display_name}\u{2800}\u{2800}\u{2800}");
        }

        let details = if settings.enable_custom_status_display {
            trim_presence_field(&display_name)
        } else if settings.show_using_ruststrap_rpc {
            "Playing with Ruststrap".to_string()
        } else {
            "Playing Roblox".to_string()
        };

        let state = if settings.enable_custom_status_display {
            trim_presence_field(&status)
        } else {
            trim_presence_field(&display_name)
        };

        let (small_image, small_text) = if settings.show_account_on_rich_presence {
            if let Some(user) = user_display {
                (
                    normalize_discord_image_key(
                        &user.headshot_url,
                        RUSTSTRAP_USER_FALLBACK_IMAGE_KEY,
                    ),
                    format!("Playing on {} (@{})", user.display_name, user.username),
                )
            } else {
                (
                    RUSTSTRAP_USER_FALLBACK_IMAGE_KEY.to_string(),
                    "Ruststrap user".to_string(),
                )
            }
        } else {
            (
                RUSTSTRAP_USER_FALLBACK_IMAGE_KEY.to_string(),
                "Ruststrap user".to_string(),
            )
        };

        let mut buttons = Vec::new();
        if !settings.hide_rpc_buttons {
            let show_join_button = match activity.server_type {
                ServerType::Public => true,
                ServerType::Reserved => !activity.rpc_launch_data.is_empty(),
                ServerType::Private => false,
            };

            if show_join_button {
                buttons.push(PresenceButton {
                    label: "Join server".to_string(),
                    url: activity.get_invite_deeplink(true),
                });
            }
        }

        buttons.push(PresenceButton {
            label: "See game page".to_string(),
            url: format!("https://www.roblox.com/games/{}", activity.place_id),
        });

        let timestamp_start = if settings.playtime_counter {
            activity
                .time_joined
                .as_ref()
                .and_then(|value| value.parse::<i64>().ok())
                .or_else(|| Some(unix_now()))
        } else {
            None
        };

        let presence = PresenceData {
            details,
            state,
            large_image_key: normalize_discord_image_key(
                icon_url,
                RUSTSTRAP_GAME_FALLBACK_IMAGE_KEY,
            ),
            large_image_text: trim_presence_field(universe_name),
            small_image_key: small_image,
            small_image_text: trim_presence_field(&small_text),
            timestamp_start,
            timestamp_end: None,
            buttons,
        };

        *self.presence.lock().unwrap() = Some(presence.clone());
        *self.original_presence.lock().unwrap() = Some(presence);
    }

    pub fn set_home_presence(&self, settings: &RpcDisplaySettings) {
        let details = "Ruststrap".to_string();
        let state = if settings.show_using_ruststrap_rpc {
            "in home • using Ruststrap".to_string()
        } else {
            "in home".to_string()
        };

        let presence = PresenceData {
            details,
            state,
            large_image_key: normalize_discord_image_key(
                RUSTSTRAP_HOME_LARGE_IMAGE_KEY,
                RUSTSTRAP_GAME_FALLBACK_IMAGE_KEY,
            ),
            large_image_text: "Ruststrap home".to_string(),
            small_image_key: normalize_discord_image_key(
                RUSTSTRAP_HOME_SMALL_IMAGE_KEY,
                RUSTSTRAP_USER_FALLBACK_IMAGE_KEY,
            ),
            small_image_text: "Ruststrap".to_string(),
            timestamp_start: None,
            timestamp_end: None,
            buttons: Vec::new(),
        };

        *self.presence.lock().unwrap() = Some(presence.clone());
        *self.original_presence.lock().unwrap() = Some(presence);
    }

    pub fn clear_presence(&self) {
        *self.presence.lock().unwrap() = None;
        *self.original_presence.lock().unwrap() = None;
    }

    pub fn process_rpc_message(&self, message: &RpcMessage) {
        if message.command != "SetRichPresence" && message.command != "SetLaunchData" {
            return;
        }

        let mut presence_guard = self.presence.lock().unwrap();
        let original_guard = self.original_presence.lock().unwrap();
        let (presence, original) = match (presence_guard.as_mut(), original_guard.as_ref()) {
            (Some(presence), Some(original)) => (presence, original),
            _ => return,
        };

        if message.command == "SetLaunchData" {
            return;
        }

        if let Some(data) = &message.data {
            if let Ok(rpc) = serde_json::from_value::<RpcRichPresence>(data.clone()) {
                if let Some(details) = &rpc.details {
                    if details.len() <= 128 {
                        if details == "<reset>" {
                            presence.details = original.details.clone();
                        } else {
                            presence.details = details.clone();
                        }
                    }
                }

                if let Some(state) = &rpc.state {
                    if state.len() <= 128 {
                        if state == "<reset>" {
                            presence.state = original.state.clone();
                        } else {
                            presence.state = state.clone();
                        }
                    }
                }

                if let Some(ts) = rpc.timestamp_start {
                    if ts == 0 {
                        presence.timestamp_start = None;
                    } else {
                        presence.timestamp_start = Some(ts);
                    }
                }

                if let Some(ts) = rpc.timestamp_end {
                    if ts == 0 {
                        presence.timestamp_end = None;
                    } else {
                        presence.timestamp_end = Some(ts);
                    }
                }

                if let Some(small) = &rpc.small_image {
                    if small.clear {
                        presence.small_image_key.clear();
                    } else if small.reset {
                        presence.small_image_key = original.small_image_key.clone();
                        presence.small_image_text = original.small_image_text.clone();
                    }
                    if let Some(text) = &small.hover_text {
                        presence.small_image_text = trim_presence_field(text);
                    }
                }

                if let Some(large) = &rpc.large_image {
                    if large.clear {
                        presence.large_image_key.clear();
                    } else if large.reset {
                        presence.large_image_key = original.large_image_key.clone();
                        presence.large_image_text = original.large_image_text.clone();
                    }
                    if let Some(text) = &large.hover_text {
                        presence.large_image_text = trim_presence_field(text);
                    }
                }
            }
        }
    }

    pub fn set_visibility(&self, visible: bool) {
        *self.visible.lock().unwrap() = visible;
    }

    pub fn get_current_presence(&self) -> Option<PresenceData> {
        let visible = *self.visible.lock().unwrap();
        if !visible {
            return None;
        }
        self.presence.lock().unwrap().clone()
    }

    pub fn app_id() -> &'static str {
        DISCORD_APP_ID
    }

    pub fn cache_thumbnail(&self, id: u64, url: &str) {
        let mut cache = self.thumbnail_cache.lock().unwrap();
        cache.retain(|entry| entry.id != id);
        cache.push(ThumbnailCacheEntry {
            id,
            url: url.to_string(),
        });
        if cache.len() > 20 {
            let _ = cache.remove(0);
        }
    }

    pub fn cached_thumbnail(&self, id: u64) -> Option<String> {
        let cache = self.thumbnail_cache.lock().unwrap();
        cache
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.url.clone())
    }
}

fn trim_presence_field(value: &str) -> String {
    value.chars().take(128).collect::<String>()
}

fn normalize_discord_image_key(value: &str, fallback: &str) -> String {
    let resolved = normalize_discord_image_key_inner(value);
    if resolved.is_empty() {
        return normalize_discord_image_key_inner(fallback);
    }
    resolved
}

fn normalize_discord_image_key_inner(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with("mp:") {
        return trimmed.to_string();
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return format!("mp:{trimmed}");
    }

    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == ':')
    {
        return trimmed.to_string();
    }

    String::new()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn fetch_user_display(user_id: i64) -> Result<Option<RobloxUserDisplay>> {
    if user_id <= 0 {
        return Ok(None);
    }

    let Some(user) = fetch_user_details(user_id)? else {
        return Ok(None);
    };

    let headshot = fetch_user_headshot_url(user_id as u64)?
        .map(|value| normalize_discord_image_key(&value, RUSTSTRAP_USER_FALLBACK_IMAGE_KEY))
        .unwrap_or_else(|| RUSTSTRAP_USER_FALLBACK_IMAGE_KEY.to_string());

    Ok(Some(RobloxUserDisplay {
        headshot_url: headshot,
        display_name: user.display_name,
        username: user.name,
    }))
}

pub fn fetch_user_details(user_id: i64) -> Result<Option<GetUserResponse>> {
    let url = format!("https://users.roblox.com/v1/users/{user_id}");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("user details client build failed: {e}")))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("user details request failed: {e}")))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("user details response read failed: {e}")))?;

    let details: GetUserResponse = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("user details parse failed: {e}")))?;

    Ok(Some(details))
}

pub fn fetch_user_headshot_url(user_id: u64) -> Result<Option<String>> {
    let requests = vec![serde_json::json!({
        "requestId": format!("0:{user_id}:AvatarHeadShot:150x150:png:regular"),
        "targetId": user_id,
        "type": "AvatarHeadShot",
        "size": "150x150",
        "isCircular": true
    })];

    fetch_thumbnail_batch_url(&requests, "headshot")
}

pub fn fetch_thumbnail_url(target_id: u64, kind: &str, size: &str) -> Result<Option<String>> {
    let requests = vec![serde_json::json!({
        "requestId": format!("0:{target_id}:Asset:512x512:png:regular"),
        "targetId": target_id,
        "type": kind,
        "size": size,
        "isCircular": false
    })];

    fetch_thumbnail_batch_url(&requests, "thumbnail")
}

fn fetch_thumbnail_batch_url(
    requests: &[serde_json::Value],
    request_kind: &str,
) -> Result<Option<String>> {
    let url = "https://thumbnails.roblox.com/v1/batch";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| {
            DomainError::Network(format!("{request_kind} thumbnail client build failed: {e}"))
        })?;

    for attempt in 0..THUMBNAIL_RETRY_ATTEMPTS {
        let response = client
            .post(url)
            .json(requests)
            .send()
            .map_err(|e| {
                DomainError::Network(format!(
                    "{request_kind} thumbnail request failed (attempt {}): {e}",
                    attempt + 1
                ))
            })?;

        let body = response.text().map_err(|e| {
            DomainError::Network(format!(
                "{request_kind} thumbnail response read failed (attempt {}): {e}",
                attempt + 1
            ))
        })?;

        let batch: ThumbnailBatchResponse = serde_json::from_str(&body).map_err(|e| {
            DomainError::Serialization(format!(
                "{request_kind} thumbnail parse failed (attempt {}): {e}",
                attempt + 1
            ))
        })?;

        if let Some(url) = select_thumbnail_url(&batch) {
            return Ok(Some(url));
        }

        if attempt + 1 < THUMBNAIL_RETRY_ATTEMPTS {
            thread::sleep(Duration::from_millis(THUMBNAIL_RETRY_DELAY_MS));
        }
    }

    Ok(None)
}

fn select_thumbnail_url(batch: &ThumbnailBatchResponse) -> Option<String> {
    let mut fallback = None;

    for item in &batch.data {
        let Some(url) = item
            .image_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if item.state.eq_ignore_ascii_case("completed") {
            return Some(url.to_string());
        }
        if fallback.is_none() {
            fallback = Some(url.to_string());
        }
    }

    fallback
}

pub fn fetch_universe_details(universe_id: i64) -> Result<Option<GameDetailData>> {
    let url = format!("https://games.roblox.com/v1/games?universeIds={universe_id}");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("api client build failed: {e}")))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("universe details request failed: {e}")))?;

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("universe details read failed: {e}")))?;

    let detail_response: GameDetailResponse = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("universe details parse failed: {e}")))?;

    Ok(detail_response.data.into_iter().next())
}

pub fn query_server_location(ip: &str) -> Result<Option<String>> {
    use crate::roblox_api::RoValraGeolocation;

    let url = format!("https://apis.rovalra.com/v1/geolocation?ip={ip}");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("geolocation client build failed: {e}")))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("geolocation request failed: {e}")))?;

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("geolocation response read failed: {e}")))?;

    let geo: RoValraGeolocation = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("geolocation parse failed: {e}")))?;

    Ok(geo.location.map(|loc| loc.display()))
}

/// tests cuz rpc = israel


#[cfg(test)]
mod tests {
    use super::*;
    use crate::roblox_api::ThumbnailResponse;

    #[test]
    fn discord_app_id() {
        assert_eq!(DiscordRichPresence::app_id(), DISCORD_APP_ID);
    }

    #[test]
    fn presence_set_and_clear() {
        let rpc = DiscordRichPresence::new();
        assert!(rpc.get_current_presence().is_none());

        let activity = ActivityData {
            place_id: 123,
            job_id: "test-job".to_string(),
            server_type: ServerType::Public,
            time_joined: Some(unix_now().to_string()),
            ..Default::default()
        };

        rpc.set_current_game(
            &activity,
            "Test Game",
            "TestCreator",
            false,
            "roblox",
            &RpcDisplaySettings {
                enable_custom_status_display: true,
                playtime_counter: true,
                ..Default::default()
            },
            None,
            None,
            None,
        );

        let presence = rpc.get_current_presence().unwrap();
        assert_eq!(presence.details, "Test Game");
        assert!(presence.state.contains("TestCreator"));

        rpc.clear_presence();
        assert!(rpc.get_current_presence().is_none());
    }

    #[test]
    fn home_presence_has_no_timer() {
        let rpc = DiscordRichPresence::new();
        rpc.set_home_presence(&RpcDisplaySettings {
            show_using_ruststrap_rpc: true,
            ..Default::default()
        });

        let presence = rpc.get_current_presence().unwrap();
        assert_eq!(presence.details, "Ruststrap");
        assert!(presence.timestamp_start.is_none());
        assert!(presence.buttons.is_empty());
        assert!(presence.large_image_key.starts_with("mp:https://"));
        assert!(presence.small_image_key.starts_with("mp:https://"));
    }

    #[test]
    fn normalize_http_image_to_mp() {
        let image = normalize_discord_image_key(
            "https://example.com/image.png",
            RUSTSTRAP_GAME_FALLBACK_IMAGE_KEY,
        );
        assert_eq!(image, "mp:https://example.com/image.png");
    }

    #[test]
    fn thumbnail_selection_prefers_completed() {
        let response = ThumbnailBatchResponse {
            data: vec![
                ThumbnailResponse {
                    target_id: 1,
                    state: "Pending".to_string(),
                    image_url: Some("https://example.com/pending.png".to_string()),
                },
                ThumbnailResponse {
                    target_id: 1,
                    state: "Completed".to_string(),
                    image_url: Some("https://example.com/completed.png".to_string()),
                },
            ],
        };

        let selected = select_thumbnail_url(&response).unwrap();
        assert_eq!(selected, "https://example.com/completed.png");
    }
}
