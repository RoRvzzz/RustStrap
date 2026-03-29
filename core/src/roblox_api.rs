/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use serde::{Deserialize, Serialize};

// w rovalra

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailRequest {
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "targetId")]
    pub target_id: u64,
    #[serde(rename = "type")]
    pub kind: String,
    pub size: String,
    #[serde(rename = "isCircular")]
    pub is_circular: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailResponse {
    #[serde(rename = "targetId")]
    pub target_id: u64,
    pub state: String,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailBatchResponse {
    pub data: Vec<ThumbnailResponse>,
}

// section: universe / game details api

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCreator {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub creator_type: String,
    #[serde(rename = "isRNVAccount", default)]
    pub is_rnv_account: bool,
    #[serde(rename = "hasVerifiedBadge", default)]
    pub has_verified_badge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetailData {
    pub id: i64,
    #[serde(rename = "rootPlaceId")]
    pub root_place_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "sourceName")]
    pub source_name: Option<String>,
    #[serde(rename = "sourceDescription")]
    pub source_description: Option<String>,
    pub creator: GameCreator,
    #[serde(rename = "price")]
    pub price: Option<i64>,
    #[serde(rename = "allowedGearGenres")]
    pub allowed_gear_genres: Option<Vec<String>>,
    #[serde(rename = "isGenreEnforced")]
    pub is_genre_enforced: Option<bool>,
    #[serde(rename = "copyingAllowed")]
    pub copying_allowed: Option<bool>,
    pub playing: Option<i64>,
    pub visits: Option<i64>,
    #[serde(rename = "maxPlayers")]
    pub max_players: Option<i32>,
    pub created: Option<String>,
    pub updated: Option<String>,
    #[serde(rename = "studioAccessToApisAllowed")]
    pub studio_access_to_apis_allowed: Option<bool>,
    #[serde(rename = "createVipServersAllowed")]
    pub create_vip_servers_allowed: Option<bool>,
    #[serde(rename = "universeAvatarType")]
    pub universe_avatar_type: Option<String>,
    pub genre: Option<String>,
    #[serde(rename = "isAllGenre")]
    pub is_all_genre: Option<bool>,
    #[serde(rename = "isFavoritedByUser")]
    pub is_favorited_by_user: Option<bool>,
    #[serde(rename = "favoritedCount")]
    pub favorited_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetailResponse {
    pub data: Vec<GameDetailData>,
}

// section: universe id lookup

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseIdEntry {
    #[serde(rename = "universeId")]
    pub universe_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseIdResponse {
    pub data: Vec<UniverseIdEntry>,
}

// section: user api

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserResponse {
    pub id: i64,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "hasVerifiedBadge", default)]
    pub has_verified_badge: bool,
    pub description: Option<String>,
    pub created: Option<String>,
    #[serde(rename = "isBanned", default)]
    pub is_banned: bool,
    #[serde(rename = "externalAppDisplayName")]
    pub external_app_display_name: Option<String>,
}

// section: client version / flag settings

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientVersionResponse {
    pub version: String,
    #[serde(rename = "clientVersionUpload")]
    pub client_version_upload: String,
    #[serde(rename = "bootstrapperVersion")]
    pub bootstrapper_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFlagSettings {
    #[serde(flatten)]
    pub flags: std::collections::HashMap<String, serde_json::Value>,
}

// section: user channel

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserChannelResponse {
    #[serde(rename = "channelName")]
    pub channel_name: String,
    #[serde(rename = "channelToken", default)]
    pub channel_token: Option<String>,
}

// section: api array response (generic)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiArrayResponse<T> {
    pub data: Vec<T>,
}

// section: ruststraprpc models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    #[serde(rename = "command")]
    pub command: String,
    #[serde(rename = "data")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRichPresence {
    pub details: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "timeStart")]
    pub timestamp_start: Option<i64>,
    #[serde(rename = "timeEnd")]
    pub timestamp_end: Option<i64>,
    #[serde(rename = "smallImage")]
    pub small_image: Option<RpcRichPresenceImage>,
    #[serde(rename = "largeImage")]
    pub large_image: Option<RpcRichPresenceImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRichPresenceImage {
    #[serde(rename = "assetId")]
    pub asset_id: Option<u64>,
    #[serde(rename = "hoverText")]
    pub hover_text: Option<String>,
    #[serde(default)]
    pub clear: bool,
    #[serde(default)]
    pub reset: bool,
}

// section: rovalra api models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraGeolocation {
    pub location: Option<RoValraServerLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraServerLocation {
    pub city: String,
    pub country_name: String,
    pub region: String,
}

impl RoValraServerLocation {
    /// format location as a human-readable string.
    pub fn display(&self) -> String {
        if self.city == self.region && self.city == self.country_name {
            self.country_name.clone()
        } else if self.city == self.region {
            format!("{}, {}", self.region, self.country_name)
        } else {
            format!("{}, {}, {}", self.city, self.region, self.country_name)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraDatacenter {
    pub name: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraDatacenters {
    pub data: Vec<RoValraDatacenter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraServerEntry {
    pub ip: String,
    pub datacenter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoValraServers {
    pub data: Vec<RoValraServerEntry>,
}

// section: github release models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Option<Vec<GitHubReleaseAsset>>,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: Option<i64>,
    pub content_type: Option<String>,
}

// section: config / remote data models

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteDataBase {
    #[serde(rename = "DeeplinkUrl", default)]
    pub deeplink_url: String,
    #[serde(rename = "SupporterData", default)]
    pub supporter_data: Option<SupporterData>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupporterData {
    pub groups: Vec<SupporterGroup>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupporterGroup {
    pub name: String,
    pub members: Vec<Supporter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Supporter {
    pub name: String,
    pub url: Option<String>,
}

// section: thumbnail cache

#[derive(Debug, Clone)]
pub struct ThumbnailCacheEntry {
    pub id: u64,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rovalra_display_same_city_region_country() {
        let loc = RoValraServerLocation {
            city: "USA".to_string(),
            region: "USA".to_string(),
            country_name: "USA".to_string(),
        };
        assert_eq!(loc.display(), "USA");
    }

    #[test]
    fn rovalra_display_city_equals_region() {
        let loc = RoValraServerLocation {
            city: "California".to_string(),
            region: "California".to_string(),
            country_name: "United States".to_string(),
        };
        assert_eq!(loc.display(), "California, United States");
    }

    #[test]
    fn rovalra_display_full() {
        let loc = RoValraServerLocation {
            city: "Ashburn".to_string(),
            region: "Virginia".to_string(),
            country_name: "United States".to_string(),
        };
        assert_eq!(loc.display(), "Ashburn, Virginia, United States");
    }

    #[test]
    fn rpc_message_deserialize() {
        let json = r#"{"command":"SetRichPresence","data":{"details":"Playing"}}"#;
        let msg: RpcMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.command, "SetRichPresence");
        assert!(msg.data.is_some());
    }
}
