use serde::{Deserialize, Serialize};

use crate::enums::ServerType;

/// tracks a single game activity session. Mirrors Ruststrap's ActivityData.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityData {
    pub universe_id: i64,
    pub place_id: i64,
    pub job_id: String,
    pub user_id: i64,
    pub machine_address: String,
    pub access_code: String,
    pub server_type: ServerType,
    pub is_teleport: bool,
    pub time_joined: Option<String>,
    pub time_left: Option<String>,
    pub start_time: Option<String>,
    pub rpc_launch_data: String,
    pub root_place_id: Option<i64>,
}

impl ActivityData {
    pub fn new() -> Self {
        Self::default()
    }

    /// validate machine address for geolocation queries
    pub fn machine_address_valid(&self) -> bool {
        !self.machine_address.is_empty() && !self.machine_address.starts_with("10.")
    }

    /// deeplink builder
    pub fn get_invite_deeplink(&self, include_launch_data: bool) -> String {
        let mut deeplink = format!("roblox://experiences/start?placeId={}", self.place_id);

        match self.server_type {
            ServerType::Private => {
                deeplink.push_str(&format!("&accessCode={}", self.access_code));
            }
            _ => {
                deeplink.push_str(&format!("&gameInstanceId={}", self.job_id));
            }
        }

        if include_launch_data && !self.rpc_launch_data.is_empty() {
            deeplink.push_str(&format!(
                "&launchData={}",
                urlencoding::encode(&self.rpc_launch_data)
            ));
        }

        deeplink
    }

    /// reset activity
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_deeplink_public_server() {
        let data = ActivityData {
            place_id: 123456,
            job_id: "abc-def".to_string(),
            server_type: ServerType::Public,
            ..Default::default()
        };
        let link = data.get_invite_deeplink(false);
        assert!(link.contains("placeId=123456"));
        assert!(link.contains("gameInstanceId=abc-def"));
    }

    #[test]
    fn invite_deeplink_private_server() {
        let data = ActivityData {
            place_id: 789,
            access_code: "secret-code".to_string(),
            server_type: ServerType::Private,
            ..Default::default()
        };
        let link = data.get_invite_deeplink(false);
        assert!(link.contains("accessCode=secret-code"));
    }

    #[test]
    fn machine_address_validation() {
        let mut data = ActivityData::default();
        assert!(!data.machine_address_valid());

        data.machine_address = "10.0.0.1".to_string();
        assert!(!data.machine_address_valid());

        data.machine_address = "128.116.0.1".to_string();
        assert!(data.machine_address_valid());
    }
}
