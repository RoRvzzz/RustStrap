use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::errors::{DomainError, Result};

/// cookie authentication state — mirrors Ruststrap's CookieState enum.
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
#[derive(Debug, Deserialize)]
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
                // validate the cookie
                match self.validate_cookie() {
                    Ok(true) => {
                        self.state = CookieState::Success;
                    }
                    Ok(false) => {
                        self.state = CookieState::Invalid;
                        self.auth_cookie = None;
                    }
                    Err(error) => {
                        self.state = classify_cookie_validation_error(&error);
                        self.auth_cookie = None;
                    }
                }
            }
            Err(error) => {
                self.state = classify_cookie_load_error(&error);
                self.auth_cookie = None;
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

    fn validate_cookie(&self) -> Result<bool> {
        let cookie = match &self.auth_cookie {
            Some(c) => c,
            None => return Ok(false),
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .get("https://users.roblox.com/v1/users/authenticated")
            .header("Cookie", format!(".ROBLOSECURITY={cookie}"))
            .send()
            .map_err(|e| DomainError::Network(format!("auth validation failed: {e}")))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let body = response
            .text()
            .map_err(|e| DomainError::Serialization(format!("auth response read failed: {e}")))?;

        let user: AuthenticatedUser = serde_json::from_str(&body)
            .map_err(|e| DomainError::Serialization(format!("auth user parse failed: {e}")))?;

        Ok(user.id != 0)
    }

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

    /// get the raw cookie value (for passing to other systems).
    pub fn cookie_value(&self) -> Option<&str> {
        self.auth_cookie.as_deref()
    }
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
}
