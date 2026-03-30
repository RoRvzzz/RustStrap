/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/


// wip feature bear with me
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::cookies::{
    authenticated_user_from_cookie, decrypt_secret_for_current_user,
    encrypt_secret_for_current_user, normalize_roblosecurity_cookie, AuthenticatedUser,
    CookiesManager,
};
use crate::errors::{DomainError, Result};

const ACCOUNT_MANAGER_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AccountManagerFile {
    schema_version: u32,
    active_account_id: Option<String>,
    accounts: Vec<StoredAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAccount {
    id: String,
    alias: String,
    user_id: u64,
    username: String,
    display_name: String,
    cookie_secret: String,
    created_at_utc: String,
    updated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountProfile {
    pub id: String,
    pub alias: String,
    pub user_id: u64,
    pub username: String,
    pub display_name: String,
    pub active: bool,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountManagerSnapshot {
    pub active_account_id: Option<String>,
    pub accounts: Vec<AccountProfile>,
}

impl Default for AccountManagerFile {
    fn default() -> Self {
        Self {
            schema_version: ACCOUNT_MANAGER_SCHEMA_VERSION,
            active_account_id: None,
            accounts: Vec::new(),
        }
    }
}

pub struct AccountManager {
    file_path: PathBuf,
}

impl AccountManager {
    pub fn from_base_dir(base_dir: &Path) -> Self {
        Self::new(base_dir.join("AccountManager.json"))
    }

    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn snapshot(&self) -> Result<AccountManagerSnapshot> {
        let state = self.load_state()?;
        Ok(self.snapshot_from_state(&state))
    }

    pub fn add_or_update_cookie(
        &self,
        cookie_raw: &str,
        alias: Option<&str>,
    ) -> Result<AccountManagerSnapshot> {
        let cookie = normalize_roblosecurity_cookie(cookie_raw);
        if cookie.is_empty() {
            return Err(DomainError::InvalidLaunchRequest(
                "ROBLOSECURITY cookie cannot be empty".to_string(),
            ));
        }

        let user = authenticated_user_from_cookie(&cookie)?.ok_or_else(|| {
            DomainError::Network("cookie is invalid or expired for Roblox auth".to_string())
        })?;

        self.upsert_account(&cookie, &user, alias)
    }

    pub fn import_current_cookie(&self) -> Result<AccountManagerSnapshot> {
        let mut manager = CookiesManager::new(true);
        manager.load_cookies()?;
        let cookie = manager.cookie_value().ok_or_else(|| {
            DomainError::Network("No valid Roblox cookie found in LocalStorage".to_string())
        })?;
        let user = manager.authenticated_user().cloned().ok_or_else(|| {
            DomainError::Network("Unable to resolve authenticated Roblox user".to_string())
        })?;

        self.upsert_account(cookie, &user, None)
    }

    pub fn rename_account(&self, id: &str, alias: &str) -> Result<AccountManagerSnapshot> {
        let mut state = self.load_state()?;
        let account = state
            .accounts
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| {
                DomainError::InvalidLaunchRequest(format!("account `{id}` was not found"))
            })?;

        account.alias = normalize_alias(alias, &account.display_name, &account.username);
        account.updated_at_utc = now_utc_string();

        self.save_state(&state)?;
        Ok(self.snapshot_from_state(&state))
    }

    pub fn set_active_account(&self, id: &str) -> Result<AccountManagerSnapshot> {
        let mut state = self.load_state()?;
        if !state.accounts.iter().any(|entry| entry.id == id) {
            return Err(DomainError::InvalidLaunchRequest(format!(
                "account `{id}` was not found"
            )));
        }
        state.active_account_id = Some(id.to_string());
        self.save_state(&state)?;
        Ok(self.snapshot_from_state(&state))
    }

    pub fn clear_active_account(&self) -> Result<AccountManagerSnapshot> {
        let mut state = self.load_state()?;
        state.active_account_id = None;
        self.save_state(&state)?;
        Ok(self.snapshot_from_state(&state))
    }

    pub fn remove_account(&self, id: &str) -> Result<AccountManagerSnapshot> {
        let mut state = self.load_state()?;
        let before = state.accounts.len();
        state.accounts.retain(|entry| entry.id != id);
        if before == state.accounts.len() {
            return Err(DomainError::InvalidLaunchRequest(format!(
                "account `{id}` was not found"
            )));
        }

        if state.active_account_id.as_deref() == Some(id) {
            state.active_account_id = state.accounts.first().map(|entry| entry.id.clone());
        }

        self.save_state(&state)?;
        Ok(self.snapshot_from_state(&state))
    }

    pub fn active_cookie_value(&self) -> Result<Option<String>> {
        let state = self.load_state()?;
        let Some(active_id) = state.active_account_id.as_deref() else {
            return Ok(None);
        };
        let Some(account) = state.accounts.iter().find(|entry| entry.id == active_id) else {
            return Ok(None);
        };

        let decrypted = decrypt_secret_for_current_user(&account.cookie_secret)?;
        let normalized = normalize_roblosecurity_cookie(&decrypted);
        if normalized.is_empty() {
            return Ok(None);
        }
        Ok(Some(normalized))
    }

    fn upsert_account(
        &self,
        cookie: &str,
        user: &AuthenticatedUser,
        alias: Option<&str>,
    ) -> Result<AccountManagerSnapshot> {
        let mut state = self.load_state()?;
        let now = now_utc_string();
        let encrypted_cookie = encrypt_secret_for_current_user(cookie)?;

        if let Some(existing) = state.accounts.iter_mut().find(|entry| entry.user_id == user.id) {
            existing.username = user.name.clone();
            existing.display_name = user.display_name.clone();
            existing.alias = normalize_alias(
                alias.unwrap_or(existing.alias.as_str()),
                &existing.display_name,
                &existing.username,
            );
            existing.cookie_secret = encrypted_cookie;
            existing.updated_at_utc = now.clone();

            if state.active_account_id.is_none() {
                state.active_account_id = Some(existing.id.clone());
            }
        } else {
            let id = generate_account_id(user.id);
            let username = user.name.clone();
            let display_name = user.display_name.clone();
            let account = StoredAccount {
                id: id.clone(),
                alias: normalize_alias(alias.unwrap_or_default(), &display_name, &username),
                user_id: user.id,
                username,
                display_name,
                cookie_secret: encrypted_cookie,
                created_at_utc: now.clone(),
                updated_at_utc: now,
            };
            state.accounts.push(account);
            if state.active_account_id.is_none() {
                state.active_account_id = Some(id);
            }
        }

        self.save_state(&state)?;
        Ok(self.snapshot_from_state(&state))
    }

    fn load_state(&self) -> Result<AccountManagerFile> {
        if !self.file_path.exists() {
            return Ok(AccountManagerFile::default());
        }

        let raw = fs::read_to_string(&self.file_path)?;
        let mut parsed: AccountManagerFile = serde_json::from_str(&raw).map_err(|error| {
            DomainError::Serialization(format!("failed to parse account manager file: {error}"))
        })?;
        if parsed.schema_version == 0 {
            parsed.schema_version = ACCOUNT_MANAGER_SCHEMA_VERSION;
        }
        Ok(parsed)
    }

    fn save_state(&self, state: &AccountManagerFile) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(state).map_err(|error| {
            DomainError::Serialization(format!("failed to serialize account manager file: {error}"))
        })?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    fn snapshot_from_state(&self, state: &AccountManagerFile) -> AccountManagerSnapshot {
        let active_id = state.active_account_id.clone();
        let mut accounts = state
            .accounts
            .iter()
            .map(|entry| AccountProfile {
                id: entry.id.clone(),
                alias: entry.alias.clone(),
                user_id: entry.user_id,
                username: entry.username.clone(),
                display_name: entry.display_name.clone(),
                active: active_id.as_deref() == Some(entry.id.as_str()),
                created_at_utc: entry.created_at_utc.clone(),
                updated_at_utc: entry.updated_at_utc.clone(),
            })
            .collect::<Vec<_>>();
        accounts.sort_by(|left, right| {
            right
                .updated_at_utc
                .cmp(&left.updated_at_utc)
                .then_with(|| left.username.cmp(&right.username))
        });

        AccountManagerSnapshot {
            active_account_id: active_id,
            accounts,
        }
    }
}

fn now_utc_string() -> String {
    Utc::now().to_rfc3339()
}

fn generate_account_id(user_id: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("acc_{user_id}_{nanos}")
}

fn normalize_alias(alias: &str, display_name: &str, username: &str) -> String {
    let alias = alias.trim();
    if !alias.is_empty() {
        return alias.to_string();
    }
    if !display_name.trim().is_empty() {
        return display_name.trim().to_string();
    }
    username.trim().to_string()
}
