/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cookies::{CookieState, CookiesManager};
use crate::discord_rpc::fetch_thumbnail_url;
use crate::errors::{DomainError, Result};
use crate::launch_handler::open_url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionSelectorStatus {
    pub cookie_state: String,
    pub has_valid_cookie: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDatacenters {
    pub regions: Vec<String>,
    pub datacenter_map: HashMap<i32, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionGameSearchEntry {
    pub universe_id: u64,
    pub root_place_id: i64,
    pub name: String,
    pub player_count: Option<i32>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionServerEntry {
    pub job_id: String,
    pub playing: i32,
    pub max_players: i32,
    pub ping: Option<f64>,
    pub fps: Option<f64>,
    pub data_center_id: Option<i32>,
    pub region: String,
    pub uptime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionServerPage {
    pub data: Vec<RegionServerEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DatacenterLocation {
    #[serde(default)]
    city: String,
    #[serde(default)]
    country: String,
}

#[derive(Debug, Deserialize)]
struct DatacenterEntry {
    #[serde(default)]
    location: DatacenterLocation,
    #[serde(rename = "dataCenterIds", default)]
    data_center_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct OmniSearchResponse {
    #[serde(rename = "searchResults", default)]
    search_results: Vec<OmniSearchGroup>,
}

#[derive(Debug, Deserialize)]
struct OmniSearchGroup {
    #[serde(default)]
    contents: Vec<OmniSearchContent>,
}

#[derive(Debug, Deserialize)]
struct OmniSearchContent {
    #[serde(rename = "universeId", default)]
    universe_id: u64,
    #[serde(rename = "rootPlaceId", default)]
    root_place_id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "playerCount", default)]
    player_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RobloxServerListResponse {
    #[serde(default)]
    data: Vec<RobloxServer>,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RobloxServer {
    #[serde(default)]
    id: String,
    #[serde(default)]
    playing: i32,
    #[serde(rename = "maxPlayers", default)]
    max_players: i32,
    #[serde(default)]
    ping: Option<f64>,
    #[serde(default)]
    fps: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RoValraServerDetailsResponse {
    #[serde(default)]
    servers: Vec<RoValraServerDetailEntry>,
}

#[derive(Debug, Deserialize)]
struct RoValraServerDetailEntry {
    #[serde(rename = "id", alias = "server_id", default)]
    id: Option<String>,
    #[serde(rename = "data_center_id", alias = "dataCenterId", default)]
    data_center_id: Option<i32>,
    #[serde(rename = "first_seen", alias = "firstSeen", default)]
    first_seen: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    region: Option<String>,
    #[serde(rename = "country_name", alias = "countryName", default)]
    country_name: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

pub fn region_selector_status(allow_cookie_access: bool) -> Result<RegionSelectorStatus> {
    let mut manager = CookiesManager::new(allow_cookie_access);
    manager.load_cookies()?;

    let state = manager.state();
    Ok(RegionSelectorStatus {
        cookie_state: cookie_state_name(state).to_string(),
        has_valid_cookie: state == CookieState::Success,
    })
}

pub fn region_selector_datacenters() -> Result<RegionDatacenters> {
    let client = build_http_client()?;
    let response = client
        .get("https://apis.rovalra.com/v1/datacenters/list")
        .send()
        .map_err(|e| DomainError::Network(format!("datacenter request failed: {e}")))?;

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("datacenter read failed: {e}")))?;

    let entries: Vec<DatacenterEntry> = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("datacenter parse failed: {e}")))?;

    let mut region_set = BTreeSet::<String>::new();
    let mut datacenter_map = HashMap::<i32, String>::new();

    for entry in entries {
        let region_name = normalize_region_name(&entry.location.city, &entry.location.country);
        region_set.insert(region_name.clone());
        for data_center_id in entry.data_center_ids {
            datacenter_map.insert(data_center_id, region_name.clone());
        }
    }

    Ok(RegionDatacenters {
        regions: region_set.into_iter().collect(),
        datacenter_map,
    })
}

pub fn region_selector_search_games(query: &str) -> Result<Vec<RegionGameSearchEntry>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let client = build_http_client()?;
    let url = format!(
        "https://apis.roblox.com/search-api/omni-search?searchQuery={}&sessionid=0&pageType=Game",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("game search request failed: {e}")))?;

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("game search read failed: {e}")))?;

    let payload: OmniSearchResponse = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("game search parse failed: {e}")))?;

    let mut seen = HashSet::<u64>::new();
    let mut results = Vec::<RegionGameSearchEntry>::new();

    for group in payload.search_results {
        for item in group.contents {
            if item.universe_id == 0 || !seen.insert(item.universe_id) {
                continue;
            }

            let thumbnail =
                fetch_thumbnail_url(item.universe_id, "GameIcon", "150x150").unwrap_or_default();

            results.push(RegionGameSearchEntry {
                universe_id: item.universe_id,
                root_place_id: item.root_place_id,
                name: item
                    .name
                    .unwrap_or_else(|| format!("Game {}", item.universe_id)),
                player_count: item.player_count,
                thumbnail_url: thumbnail,
            });

            if results.len() >= 10 {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

pub fn region_selector_servers(
    place_id: i64,
    cursor: Option<&str>,
    sort_order: Option<i32>,
    selected_region: Option<&str>,
) -> Result<RegionServerPage> {
    let client = build_http_client()?;
    let datacenters = region_selector_datacenters().unwrap_or_else(|_| RegionDatacenters {
        regions: Vec::new(),
        datacenter_map: HashMap::new(),
    });

    let cursor = cursor.unwrap_or_default();
    let sort_order = sort_order.unwrap_or(2);
    let url = format!(
        "https://games.roblox.com/v1/games/{place_id}/servers/Public?sortOrder={sort_order}&excludeFullGames=true&limit=100&cursor={}",
        urlencoding::encode(cursor)
    );

    let response = client
        .get(&url)
        .send()
        .map_err(|e| DomainError::Network(format!("server list request failed: {e}")))?;

    let body = response
        .text()
        .map_err(|e| DomainError::Network(format!("server list read failed: {e}")))?;

    let server_page: RobloxServerListResponse = serde_json::from_str(&body)
        .map_err(|e| DomainError::Serialization(format!("server list parse failed: {e}")))?;

    let job_ids = server_page
        .data
        .iter()
        .map(|server| server.id.clone())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();

    let details_map = fetch_server_detail_map(&client, place_id, &job_ids).unwrap_or_default();

    let region_filter = selected_region
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    let mut out = Vec::<RegionServerEntry>::new();
    for server in server_page.data {
        if server.id.is_empty() {
            continue;
        }

        let detail = details_map.get(&server.id);
        let data_center_id = detail.and_then(|value| value.data_center_id);
        let region_from_datacenter = data_center_id
            .and_then(|id| datacenters.datacenter_map.get(&id).cloned())
            .filter(|value| !value.trim().is_empty() && !value.eq_ignore_ascii_case("unknown"));
        let region = region_from_datacenter
            .or_else(|| detail.and_then(resolve_region_from_detail))
            .unwrap_or_else(|| "Unknown".to_string());

        if let Some(filter) = &region_filter {
            if region.to_ascii_lowercase() != *filter {
                continue;
            }
        }

        let uptime = detail
            .and_then(|value| value.first_seen.as_deref())
            .and_then(format_uptime);

        out.push(RegionServerEntry {
            job_id: server.id,
            playing: server.playing,
            max_players: server.max_players,
            ping: server.ping,
            fps: server.fps,
            data_center_id,
            region,
            uptime,
        });
    }

    Ok(RegionServerPage {
        data: out,
        next_cursor: server_page.next_page_cursor,
    })
}

pub fn region_selector_join(place_id: i64, job_id: &str) -> Result<()> {
    let job_id = job_id.trim();
    if place_id <= 0 || job_id.is_empty() {
        return Err(DomainError::InvalidLaunchRequest(
            "place_id and job_id are required".to_string(),
        ));
    }

    let deeplink = format!(
        "roblox://experiences/start?placeId={place_id}&gameInstanceId={}",
        urlencoding::encode(job_id)
    );
    open_url(&deeplink)
}

fn fetch_server_detail_map(
    client: &reqwest::blocking::Client,
    place_id: i64,
    job_ids: &[String],
) -> Result<HashMap<String, RoValraServerDetailEntry>> {
    if job_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // kick rovalra enrichment first so first_seen / datacenter metadata is more likely
    let _ = request_server_detail_enrichment(client, place_id, job_ids);

    let joined_ids = job_ids.join(",");
    let url = format!(
        "https://apis.rovalra.com/v1/server_details?place_id={place_id}&server_ids={}",
        urlencoding::encode(&joined_ids)
    );

    let max_attempts = 3usize;
    let mut last_error: Option<DomainError> = None;

    for attempt in 0..max_attempts {
        let response = match client.get(&url).send() {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(DomainError::Network(format!(
                    "server details request failed: {error}"
                )));
                if attempt + 1 < max_attempts {
                    std::thread::sleep(std::time::Duration::from_millis(
                        300 * (attempt as u64 + 1),
                    ));
                    continue;
                }
                break;
            }
        };

        let body = match response.text() {
            Ok(body) => body,
            Err(error) => {
                last_error = Some(DomainError::Network(format!(
                    "server details read failed: {error}"
                )));
                if attempt + 1 < max_attempts {
                    std::thread::sleep(std::time::Duration::from_millis(
                        300 * (attempt as u64 + 1),
                    ));
                    continue;
                }
                break;
            }
        };

        let payload: RoValraServerDetailsResponse = match serde_json::from_str(&body) {
            Ok(payload) => payload,
            Err(error) => {
                last_error = Some(DomainError::Serialization(format!(
                    "server details parse failed: {error}"
                )));
                if attempt + 1 < max_attempts {
                    std::thread::sleep(std::time::Duration::from_millis(
                        300 * (attempt as u64 + 1),
                    ));
                    continue;
                }
                break;
            }
        };

        let mut out = HashMap::<String, RoValraServerDetailEntry>::new();
        for (index, detail) in payload.servers.into_iter().enumerate() {
            let key = detail
                .id
                .clone()
                .unwrap_or_else(|| job_ids.get(index).cloned().unwrap_or_else(String::new));
            if key.is_empty() {
                continue;
            }
            out.insert(key, detail);
        }

        if !out.is_empty() || attempt + 1 >= max_attempts {
            return Ok(out);
        }

        std::thread::sleep(std::time::Duration::from_millis(300 * (attempt as u64 + 1)));
    }

    Err(last_error.unwrap_or_else(|| {
        DomainError::Network("server details request failed after retries".to_string())
    }))
}

fn request_server_detail_enrichment(
    client: &reqwest::blocking::Client,
    place_id: i64,
    job_ids: &[String],
) -> Result<()> {
    let payload = serde_json::json!({
        "place_id": place_id,
        "server_ids": job_ids,
    });

    client
        .post("https://apis.rovalra.com/process_servers")
        .json(&payload)
        .send()
        .map_err(|e| DomainError::Network(format!("process_servers request failed: {e}")))?;

    Ok(())
}

fn resolve_region_from_detail(detail: &RoValraServerDetailEntry) -> Option<String> {
    let city = detail.city.as_deref().unwrap_or_default().trim();
    let country = detail
        .country_name
        .as_deref()
        .unwrap_or_else(|| detail.country.as_deref().unwrap_or_default())
        .trim();
    let region = detail.region.as_deref().unwrap_or_default().trim();

    if !city.is_empty() || !country.is_empty() {
        let value = normalize_region_name(city, country);
        if !value.eq_ignore_ascii_case("unknown") {
            return Some(value);
        }
    }

    if !region.is_empty() {
        return Some(region.to_string());
    }

    None
}

fn format_uptime(first_seen: &str) -> Option<String> {
    let started = DateTime::parse_from_rfc3339(first_seen)
        .ok()?
        .with_timezone(&Utc);
    let delta = Utc::now() - started;
    if delta.num_seconds() < 0 {
        return None;
    }

    let seconds = delta.num_seconds() as u64;
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        Some(format!("{days}d {hours}h {minutes}m"))
    } else if hours > 0 {
        Some(format!("{hours}h {minutes}m"))
    } else {
        Some(format!("{minutes}m"))
    }
}

fn normalize_region_name(city: &str, country: &str) -> String {
    let city = city.trim();
    let country = country.trim();

    match (city.is_empty(), country.is_empty()) {
        (true, true) => "Unknown".to_string(),
        (false, true) => city.to_string(),
        (true, false) => country.to_string(),
        (false, false) => format!("{city}, {country}"),
    }
}

fn cookie_state_name(state: CookieState) -> &'static str {
    match state {
        CookieState::Unknown => "unknown",
        CookieState::NotAllowed => "not_allowed",
        CookieState::NotFound => "not_found",
        CookieState::Invalid => "invalid",
        CookieState::Failed => "failed",
        CookieState::Success => "success",
    }
}

fn build_http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Ruststrap/0.1")
        .build()
        .map_err(|e| DomainError::Network(format!("http client build failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_name_formatting() {
        assert_eq!(normalize_region_name("", ""), "Unknown");
        assert_eq!(normalize_region_name("Ashburn", ""), "Ashburn");
        assert_eq!(normalize_region_name("", "United States"), "United States");
        assert_eq!(
            normalize_region_name("Ashburn", "United States"),
            "Ashburn, United States"
        );
    }

    #[test]
    fn cookie_state_names_are_stable() {
        assert_eq!(cookie_state_name(CookieState::Success), "success");
        assert_eq!(cookie_state_name(CookieState::NotFound), "not_found");
    }

    #[test]
    fn rovalra_server_detail_aliases_parse() {
        let payload = r#"{
            "servers": [{
                "server_id": "job-1",
                "data_center_id": 468,
                "first_seen": "2026-03-24T22:38:26.813426Z",
                "city": "Chicago",
                "country_name": "United States"
            }]
        }"#;
        let parsed: RoValraServerDetailsResponse = serde_json::from_str(payload).unwrap();
        let server = parsed.servers.first().unwrap();
        assert_eq!(server.id.as_deref(), Some("job-1"));
        assert_eq!(server.data_center_id, Some(468));
        assert_eq!(
            server.first_seen.as_deref(),
            Some("2026-03-24T22:38:26.813426Z")
        );
    }
}
