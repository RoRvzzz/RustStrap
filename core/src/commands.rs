/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchTarget {
    Player,
    Studio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub target: LaunchTarget,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Install,
    Uninstall,
    LaunchPlayer {
        #[serde(default)]
        args: Vec<String>,
    },
    LaunchStudio {
        #[serde(default)]
        args: Vec<String>,
    },
    OpenSettings,
    RunWatcher,
    RunBackgroundUpdater,
    CheckUpdates,
    ApplyModifications,
}
