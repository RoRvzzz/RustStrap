/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    ConfirmLaunch,
    ChannelChange,
    UpdateAvailable,
    InstallLocationMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum WatcherEvent {
    Started,
    Stopped,
    ActivityChanged { description: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DomainEvent {
    BootstrapStatus { message: String },
    Progress { current: u64, total: u64 },
    PromptRequired { kind: PromptKind, message: String },
    ConnectivityError { title: String, description: String },
    FatalError { code: String, message: String },
    WatcherActivity { activity: WatcherEvent },
}
