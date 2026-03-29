
/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::enums::GameJoinType;

/// parsed data from a roblox-player: launch URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameJoinData {
    pub join_type: GameJoinType,
    pub place_id: i64,
    pub job_id: String,
    pub access_code: String,
    pub user_id: i64,
    pub join_origin: String,
}

fn regex_match_long(url: &str, query: &str, pattern: &str) -> i64 {
    let combined = format!("{query}{pattern}");
    let re = match Regex::new(&combined) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    match re.captures(url) {
        Some(caps) => caps
            .get(1)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0),
        None => 0,
    }
}

fn regex_match_string(url: &str, query: &str, pattern: &str) -> String {
    let combined = format!("{query}{pattern}");
    let re = match Regex::new(&combined) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    match re.captures(url) {
        Some(caps) => caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
        None => String::new(),
    }
}

/// parse a `roblox-player:` launch command into structured join data.

pub fn parse_launch_command(launch_command: &str) -> GameJoinData {
    let mut join_data = GameJoinData::default();

    if !launch_command.starts_with("roblox-player:") {
        return join_data;
    }

    let placelauncher_pattern = r"placelauncherurl:(.+?)(\+|$)";
    let request_type_pattern = r"request=(.+?)&";
    let common_int_pattern = r"([0-9]+)";
    let common_id_pattern = r"([a-zA-Z0-9\-]+?)(&|\+|$)";

    let re_url = match Regex::new(placelauncher_pattern) {
        Ok(r) => r,
        Err(_) => return join_data,
    };

    let url_match = match re_url.captures(launch_command) {
        Some(caps) if caps.len() >= 3 => caps,
        _ => return join_data,
    };

    let url_encoded = &url_match[1];
    let url = match urlencoding::decode(url_encoded) {
        Ok(decoded) => decoded.to_string(),
        Err(_) => return join_data,
    };

    if url.is_empty() {
        return join_data;
    }

    let re_type = match Regex::new(request_type_pattern) {
        Ok(r) => r,
        Err(_) => return join_data,
    };

    let type_match = match re_type.captures(&url) {
        Some(caps) if caps.len() >= 2 => caps,
        _ => return join_data,
    };

    match &type_match[1] {
        "RequestGame" => {
            join_data.join_type = GameJoinType::RequestGame;
            let join_origin = regex_match_string(&url, "joinAttemptOrigin=", common_id_pattern);
            let place_id = regex_match_long(&url, "placeId=", common_int_pattern);
            if place_id == 0 {
                return join_data;
            }
            join_data.place_id = place_id;
            join_data.join_origin = join_origin;
        }
        "RequestGameJob" => {
            join_data.join_type = GameJoinType::RequestGameJob;
            let join_origin = regex_match_string(&url, "joinAttemptOrigin=", common_id_pattern);
            let job_id = regex_match_string(&url, "gameId=", common_id_pattern);
            let place_id = regex_match_long(&url, "placeId=", common_int_pattern);
            if job_id.is_empty() || place_id == 0 {
                return join_data;
            }
            join_data.place_id = place_id;
            join_data.job_id = job_id;
            join_data.join_origin = join_origin;
        }
        "RequestPrivateGame" => {
            join_data.join_type = GameJoinType::RequestPrivateGame;
            let access_code = regex_match_string(&url, "accessCode=", common_id_pattern);
            let place_id = regex_match_long(&url, "placeId=", common_int_pattern);
            if access_code.is_empty() || place_id == 0 {
                return join_data;
            }
            join_data.place_id = place_id;
            join_data.access_code = access_code;
        }
        "RequestFollowUser" => {
            join_data.join_type = GameJoinType::RequestFollowUser;
            let user_id = regex_match_long(&url, "userId=", common_int_pattern);
            if user_id == 0 {
                return join_data;
            }
            join_data.user_id = user_id;
        }
        "RequestPlayTogetherGame" => {
            join_data.join_type = GameJoinType::RequestPlayTogetherGame;
            let place_id = regex_match_long(&url, "placeId=", common_int_pattern);
            let conversation_id = regex_match_string(&url, "conversationId=", common_id_pattern);
            if conversation_id.is_empty() || place_id == 0 {
                return join_data;
            }
            join_data.place_id = place_id;
            join_data.job_id = conversation_id;
        }
        _ => {}
    }

    join_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_game() {
        let cmd = "roblox-player:1+launchmode:play+placelauncherurl:https%3A%2F%2Fassetgame.roblox.com%2Fgame%2FPlaceLauncher.ashx%3Frequest%3DRequestGame%26placeId%3D123456%26joinAttemptOrigin%3DPlayButton+";
        let data = parse_launch_command(cmd);
        assert_eq!(data.join_type, GameJoinType::RequestGame);
        assert_eq!(data.place_id, 123456);
        assert_eq!(data.join_origin, "PlayButton");
    }

    #[test]
    fn parse_empty_returns_unknown() {
        let data = parse_launch_command("");
        assert_eq!(data.join_type, GameJoinType::Unknown);
    }

    #[test]
    fn parse_non_roblox_protocol() {
        let data = parse_launch_command("https://www.roblox.com");
        assert_eq!(data.join_type, GameJoinType::Unknown);
    }
}
