/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::errors::{DomainError, Result};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookieState {
    Unknown,
    NotAllowed,
    NotFound,
    Invalid,
    Failed,
    Success,
}

/// represents the Roblox cookies file format.
#[derive(Debug, Deserialize, Serialize)]
struct RobloxCookies {
    #[serde(rename = "CookiesVersion", alias = "Version")]
    version: String,
    #[serde(rename = "CookiesData", alias = "Cookies")]
    cookies: String,
}

/// authenticated user response from Roblox API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: u64,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

/// manages loading and validating Roblox authentication cookies.
/// reads from `%LOCALAPPDATA%\Roblox\LocalStorage\RobloxCookies.dat`.
pub struct CookiesManager {
    state: CookieState,
    auth_cookie: Option<String>,
    auth_user: Option<AuthenticatedUser>,
    cookie_path: PathBuf,
    allow_access: bool,
}

impl CookiesManager {
    pub fn new(allow_access: bool) -> Self {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        let cookie_path = PathBuf::from(local_app_data)
            .join("Roblox")
            .join("LocalStorage")
            .join("RobloxCookies.dat");
        Self {
            state: CookieState::Unknown,
            auth_cookie: None,
            auth_user: None,
            cookie_path,
            allow_access,
        }
    }

    pub fn state(&self) -> CookieState {
        self.state
    }

    pub fn is_loaded(&self) -> bool {
        self.allow_access && self.state == CookieState::Success
    }

    /// load and decrypt cookies from disk using DPAPI.
    /// returns Ok(()) even if cookies can't be loaded (sets state accordingly).
    pub fn load_cookies(&mut self) -> Result<()> {
        if !self.allow_access {
            self.state = CookieState::NotAllowed;
            return Ok(());
        }

        if self.auth_cookie.is_some() {
            // already loaded
            return Ok(());
        }

        if !self.cookie_path.exists() {
            self.state = CookieState::NotFound;
            return Ok(());
        }

        match self.try_load_and_decrypt() {
            Ok(cookie) => {
                self.auth_cookie = Some(cookie);
                // validate the cookie and resolve user profile
                match self.fetch_authenticated_user() {
                    Ok(Some(user)) => {
                        self.state = CookieState::Success;
                        self.auth_user = Some(user);
                    }
                    Ok(None) => {
                        self.state = CookieState::Invalid;
                        self.auth_cookie = None;
                        self.auth_user = None;
                    }
                    Err(error) => {
                        self.state = classify_cookie_validation_error(&error);
                        self.auth_cookie = None;
                        self.auth_user = None;
                    }
                }
            }
            Err(error) => {
                self.state = classify_cookie_load_error(&error);
                self.auth_cookie = None;
                self.auth_user = None;
            }
        }

        Ok(())
    }

    fn try_load_and_decrypt(&self) -> Result<String> {
        let content = fs::read_to_string(&self.cookie_path)?;
        let cookies: RobloxCookies = serde_json::from_str(&content)
            .map_err(|e| DomainError::Serialization(format!("cookies parse failed: {e}")))?;

        if cookies.version != "1" {
            log::warn!("Unknown cookie version: {}", cookies.version);
        }

        // base64 decode the encrypted data
        let encrypted_data = base64_decode(&cookies.cookies)?;

        // decrypt using DPAPI (Windows only)
        let decrypted_data = dpapi_unprotect(&encrypted_data)?;

        let raw_cookies = String::from_utf8(decrypted_data)
            .map_err(|e| DomainError::Serialization(format!("cookies decode failed: {e}")))?;

        // extract .ROBLOSECURITY cookie using regex
        let pattern = Regex::new(r"\t\.ROBLOSECURITY\t(.+?)(;|$)")
            .map_err(|e| DomainError::Serialization(format!("regex compile failed: {e}")))?;

        let captures = pattern.captures(&raw_cookies).ok_or_else(|| {
            DomainError::Serialization("ROBLOSECURITY cookie not found".to_string())
        })?;

        Ok(captures[1].to_string())
    }

    fn fetch_authenticated_user(&self) -> Result<Option<AuthenticatedUser>> {
        let cookie = match &self.auth_cookie {
            Some(c) => c,
            None => return Ok(None),
        };
        fetch_authenticated_user_for_cookie(cookie)
    }

    pub fn authenticated_user(&self) -> Option<&AuthenticatedUser> {
        self.auth_user.as_ref()
    }

    /// get the raw cookie value (for passing to other systems).
    pub fn cookie_value(&self) -> Option<&str> {
        self.auth_cookie.as_deref()
    }
}

fn fetch_authenticated_user_for_cookie(cookie: &str) -> Result<Option<AuthenticatedUser>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get("https://users.roblox.com/v1/users/authenticated")
            .header("Cookie", format!(".ROBLOSECURITY={cookie}"))
            .send()
            .map_err(|e| DomainError::Network(format!("auth validation failed: {e}")))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let body = response
            .text()
            .map_err(|e| DomainError::Serialization(format!("auth response read failed: {e}")))?;

        let user: AuthenticatedUser = serde_json::from_str(&body)
            .map_err(|e| DomainError::Serialization(format!("auth user parse failed: {e}")))?;

        if user.id == 0 {
            return Ok(None);
        }

        Ok(Some(user))
}

impl CookiesManager {
    /// make an authenticated GET request to a Roblox API endpoint.
    pub fn auth_get(&self, url: &str) -> Result<(u16, String)> {
        let cookie = self
            .auth_cookie
            .as_ref()
            .ok_or_else(|| DomainError::Network("Cookie not loaded".to_string()))?;

        // validate host
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| DomainError::Network(format!("invalid url: {e}")))?;
        let host = parsed.host_str().unwrap_or("");
        if !host.eq_ignore_ascii_case("roblox.com") && !host.ends_with(".roblox.com") {
            return Err(DomainError::Network(
                "Host must end with roblox.com".to_string(),
            ));
        }

        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url)
            .header("Cookie", format!(".ROBLOSECURITY={cookie}"))
            .send()
            .map_err(|e| DomainError::Network(format!("auth request failed: {e}")))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .map_err(|e| DomainError::Network(format!("response read failed: {e}")))?;

        Ok((status, body))
    }

    /// make an authenticated POST request.
    pub fn auth_post(&self, url: &str, body: &str) -> Result<(u16, String)> {
        let cookie = self
            .auth_cookie
            .as_ref()
            .ok_or_else(|| DomainError::Network("Cookie not loaded".to_string()))?;

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(url)
            .header("Cookie", format!(".ROBLOSECURITY={cookie}"))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(|e| DomainError::Network(format!("auth post failed: {e}")))?;

        let status = response.status().as_u16();
        let resp_body = response
            .text()
            .map_err(|e| DomainError::Network(format!("response read failed: {e}")))?;

        Ok((status, resp_body))
    }
}

/// normalize any user-provided cookie representation to a raw ROBLOSECURITY value.
pub fn normalize_roblosecurity_cookie(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();
    let mut token = trimmed;

    if let Some(index) = lower.find(".roblosecurity=") {
        token = &trimmed[index + ".ROBLOSECURITY=".len()..];
    } else if let Some(index) = lower.find("roblosecurity=") {
        token = &trimmed[index + "ROBLOSECURITY=".len()..];
    }

    if let Some(end) = token.find(';') {
        token = &token[..end];
    }

    token.trim().trim_matches('"').trim_matches('\'').to_string()
}

/// persist a Roblox auth cookie into `%LOCALAPPDATA%\\Roblox\\LocalStorage\\RobloxCookies.dat`.
pub fn persist_roblosecurity_cookie(raw: &str) -> Result<()> {
    let cookie = normalize_roblosecurity_cookie(raw);
    if cookie.is_empty() {
        return Err(DomainError::Serialization(
            "ROBLOSECURITY cookie is empty".to_string(),
        ));
    }

    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| DomainError::Process("LOCALAPPDATA is not available".to_string()))?;
    let cookie_path = PathBuf::from(local_app_data)
        .join("Roblox")
        .join("LocalStorage")
        .join("RobloxCookies.dat");
    if let Some(parent) = cookie_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Netscape-style cookie line; Roblox only needs ROBLOSECURITY present.
    let cookies_data = format!(".roblox.com\tTRUE\t/\tFALSE\t0\t.ROBLOSECURITY\t{cookie};\n");
    let encrypted_data = dpapi_protect(cookies_data.as_bytes())?;
    let payload = RobloxCookies {
        version: "1".to_string(),
        cookies: base64_encode(&encrypted_data),
    };
    let json = serde_json::to_string_pretty(&payload)
        .map_err(|e| DomainError::Serialization(format!("cookies serialize failed: {e}")))?;
    fs::write(cookie_path, json)?;
    Ok(())
}

fn classify_cookie_load_error(error: &DomainError) -> CookieState {
    match error {
        // parse/decode errors usually mean cookie data is malformed or stale
        DomainError::Serialization(_) => CookieState::Invalid,
        _ => CookieState::Failed,
    }
}

fn classify_cookie_validation_error(_error: &DomainError) -> CookieState {
    // validation request failures are service/connectivity failures, not invalid file format
    CookieState::Failed
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // simple base64 decode without external crate
    use std::io::Read;
    let mut decoded = Vec::new();
    let mut chars = input.bytes().filter(|b| !b.is_ascii_whitespace());
    let table: Vec<u8> =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".to_vec();

    let mut buf = [0u8; 4];
    loop {
        let mut count = 0;
        for i in 0..4 {
            match chars.next() {
                Some(b'=') | None => {
                    buf[i] = 0;
                }
                Some(b) => {
                    buf[i] = table.iter().position(|&c| c == b).unwrap_or(0) as u8;
                    count = i + 1;
                }
            }
        }
        if count == 0 {
            break;
        }
        decoded.push((buf[0] << 2) | (buf[1] >> 4));
        if count > 2 {
            decoded.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if count > 3 {
            decoded.push((buf[2] << 6) | buf[3]);
        }
        if count < 4 {
            break;
        }
    }

    let _ = decoded.as_slice().read(&mut []);
    Ok(decoded)
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        let i0 = ((triple >> 18) & 0x3f) as usize;
        let i1 = ((triple >> 12) & 0x3f) as usize;
        let i2 = ((triple >> 6) & 0x3f) as usize;
        let i3 = (triple & 0x3f) as usize;

        output.push(TABLE[i0] as char);
        output.push(TABLE[i1] as char);
        output.push(if chunk.len() > 1 {
            TABLE[i2] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[i3] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(windows)]
fn dpapi_protect(plain: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input_blob = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_ptr() as *mut u8,
        };
        let mut output_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let result = CryptProtectData(
            &mut input_blob,
            None,
            None,
            None,
            None,
            0,
            &mut output_blob,
        );
        if result.is_err() {
            return Err(DomainError::Process(
                "DPAPI CryptProtectData failed".to_string(),
            ));
        }

        let encrypted =
            std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec();

        // free the output buffer using Win32 LocalFree via raw FFI
        extern "system" {
            fn LocalFree(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }
        LocalFree(output_blob.pbData as *mut std::ffi::c_void);

        Ok(encrypted)
    }
}

/// decrypt data using Windows DPAPI (CryptUnprotectData).
#[cfg(windows)]
fn dpapi_unprotect(encrypted: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input_blob = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut output_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let result =
            CryptUnprotectData(&mut input_blob, None, None, None, None, 0, &mut output_blob);

        if result.is_err() {
            return Err(DomainError::Process(
                "DPAPI CryptUnprotectData failed".to_string(),
            ));
        }

        let decrypted =
            std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec();

        // free the output buffer using Win32 LocalFree via raw FFI
        extern "system" {
            fn LocalFree(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }
        LocalFree(output_blob.pbData as *mut std::ffi::c_void);

        Ok(decrypted)
    }
}

#[cfg(not(windows))]
fn dpapi_unprotect(_encrypted: &[u8]) -> Result<Vec<u8>> {
    Err(DomainError::Process(
        "DPAPI is only available on Windows".to_string(),
    ))
}

#[cfg(not(windows))]
fn dpapi_protect(_plain: &[u8]) -> Result<Vec<u8>> {
    Err(DomainError::Process(
        "DPAPI is only available on Windows".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_wire_format_supports_current_keys() {
        let payload = r#"{"CookiesVersion":"1","CookiesData":"abc"}"#;
        let parsed: RobloxCookies = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.version, "1");
        assert_eq!(parsed.cookies, "abc");
    }

    #[test]
    fn cookie_wire_format_supports_legacy_keys() {
        let payload = r#"{"Version":"1","Cookies":"abc"}"#;
        let parsed: RobloxCookies = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.version, "1");
        assert_eq!(parsed.cookies, "abc");
    }

    #[test]
    fn malformed_cookie_content_is_marked_invalid() {
        let state = classify_cookie_load_error(&DomainError::Serialization("bad".to_string()));
        assert_eq!(state, CookieState::Invalid);
    }

    #[test]
    fn normalize_cookie_accepts_raw_cookie_header_forms() {
        let value = normalize_roblosecurity_cookie(
            ".ROBLOSECURITY=_|WARNING:-DO-NOT-SHARE-THIS.|_abc123; domain=.roblox.com; path=/",
        );
        assert_eq!(value, "_|WARNING:-DO-NOT-SHARE-THIS.|_abc123");
    }

    #[test]
    fn base64_round_trip_matches_decoder() {
        let input = b"ruststrap-cookie-test";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded).expect("decode");
        assert_eq!(decoded, input);
    }
}
