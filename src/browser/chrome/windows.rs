use super::ChromiumBrowser;
use crate::browser::{Cookie, CookieStore};
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use dirs::home_dir;
use rusqlite::{Connection, Row};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use winapi::um::dpapi::CryptUnprotectData;
use winapi::um::winbase::LocalFree;
use winapi::um::wincrypt::DATA_BLOB;

use crate::utils::FileUtils;

const WINDOWS_V10_PREFIX: &[u8; 3] = b"v10";
const WINDOWS_DPAPI_PREFIX: &[u8] = b"DPAPI";
const AES_GCM_NONCE_LEN: usize = 12;
const AES_GCM_TAG_LEN: usize = 16;

struct ChromiumSettings {
    user_data_dir: PathBuf,
    supports_profiles: bool,
}

pub fn extract_chromium_cookies(
    browser: ChromiumBrowser,
    config: &BrowserCookieConfig,
) -> Result<CookieStore> {
    let settings = chromium_settings(browser)?;
    let profile = config.profile.as_deref();
    let cookie_db = find_cookie_database(&settings, profile)?;

    let temp_dir = tempdir()
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to create temp dir: {}", e)))?;
    let temp_db = temp_dir.path().join("chromium-cookies.sqlite");
    fs::copy(&cookie_db, &temp_db).map_err(|e| {
        crate::browser::map_cookie_io_error(
            "Failed to copy cookies DB",
            &cookie_db,
            e,
            Some("Close the browser or run without elevation."),
        )
    })?;

    let conn = Connection::open(&temp_db)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to open cookies DB: {}", e)))?;
    let meta_version = read_meta_version(&conn);
    let column_names = read_cookie_columns(&conn)?;
    let secure_column = if column_names.contains("is_secure") {
        "is_secure"
    } else {
        "secure"
    };
    let httponly_column = if column_names.contains("is_httponly") {
        Some("is_httponly")
    } else if column_names.contains("httponly") {
        Some("httponly")
    } else {
        None
    };

    let decryptor = WindowsChromeCookieDecryptor::new(&settings, meta_version)?;
    let mut store: CookieStore = CookieStore::new();

    let query = if let Some(httponly) = httponly_column {
        format!(
            "SELECT host_key, name, value, encrypted_value, path, expires_utc, {}, {} FROM cookies",
            secure_column, httponly
        )
    } else {
        format!(
            "SELECT host_key, name, value, encrypted_value, path, expires_utc, {}, 0 FROM cookies",
            secure_column
        )
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to prepare cookie query: {}", e)))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to query cookies: {}", e)))?;

    while let Some(row) = rows
        .next()
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie row: {}", e)))?
    {
        if let Some(cookie) = row_to_cookie(row, &decryptor)? {
            store.entry(cookie.domain.clone()).or_default().push(cookie);
        }
    }

    if store.is_empty() {
        return Err(RurlError::BrowserCookie(
            "No Chromium cookies could be extracted".to_string(),
        ));
    }

    Ok(store)
}

fn chromium_settings(browser: ChromiumBrowser) -> Result<ChromiumSettings> {
    let local_appdata = env::var("LOCALAPPDATA").ok();
    let appdata = env::var("APPDATA").ok();
    let home = home_dir();

    let local_root = local_appdata
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join("AppData/Local")))
        .ok_or_else(|| RurlError::Config("Cannot determine LOCALAPPDATA".to_string()))?;
    let roaming_root = appdata
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join("AppData/Roaming")))
        .ok_or_else(|| RurlError::Config("Cannot determine APPDATA".to_string()))?;

    let (relative_dir, supports_profiles) = match browser {
        ChromiumBrowser::Chrome => ("Google/Chrome/User Data", true),
        ChromiumBrowser::Edge => ("Microsoft/Edge/User Data", true),
        ChromiumBrowser::Brave => ("BraveSoftware/Brave-Browser/User Data", true),
        ChromiumBrowser::Opera => ("Opera Software/Opera Stable", false),
        ChromiumBrowser::Vivaldi => ("Vivaldi/User Data", true),
        ChromiumBrowser::Whale => ("Naver/Naver Whale/User Data", true),
    };

    let user_data_dir = if matches!(browser, ChromiumBrowser::Opera) {
        roaming_root.join(relative_dir)
    } else {
        local_root.join(relative_dir)
    };

    Ok(ChromiumSettings {
        user_data_dir,
        supports_profiles,
    })
}

fn find_cookie_database(settings: &ChromiumSettings, profile: Option<&str>) -> Result<PathBuf> {
    let search_root = if let Some(profile) = profile {
        if is_path_like(profile) {
            let expanded = FileUtils::expand_path(profile)?;
            if expanded.is_file() {
                return Ok(expanded);
            }
            expanded
        } else if settings.supports_profiles {
            settings.user_data_dir.join(profile)
        } else {
            log::warn!("Profile selection is not supported for this browser");
            settings.user_data_dir.clone()
        }
    } else {
        settings.user_data_dir.clone()
    };

    if !search_root.exists() {
        return Err(RurlError::FileNotFound(format!(
            "Browser data dir not found: {:?}",
            search_root
        )));
    }

    let candidates = find_files(&search_root, "Cookies")?;
    let newest = newest_path(candidates);
    newest.ok_or_else(|| RurlError::FileNotFound("Chrome cookies database not found".to_string()))
}

fn is_path_like(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.starts_with('~')
}

fn find_files(root: &Path, filename: &str) -> Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read directory {:?}: {}", dir, e))
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                RurlError::BrowserCookie(format!(
                    "Failed to read directory entry in {:?}: {}",
                    dir, e
                ))
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(filename) {
                matches.push(path);
            }
        }
    }
    Ok(matches)
}

fn newest_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths
        .into_iter()
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn read_meta_version(conn: &Connection) -> i64 {
    let result: std::result::Result<String, _> =
        conn.query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
            row.get(0)
        });
    result
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn read_cookie_columns(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(cookies)")
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e)))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e)))?;
    let mut columns = HashSet::new();
    for row in rows {
        let name = row.map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e))
        })?;
        columns.insert(name);
    }
    Ok(columns)
}

fn row_to_cookie(
    row: &Row<'_>,
    decryptor: &WindowsChromeCookieDecryptor,
) -> Result<Option<Cookie>> {
    let host_key: String = row
        .get(0)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie host: {}", e)))?;
    let name: String = row
        .get(1)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie name: {}", e)))?;
    let value: String = row
        .get(2)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie value: {}", e)))?;
    let encrypted_value = read_encrypted_value(row)?;
    let path: String = row
        .get(4)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie path: {}", e)))?;
    let expires_utc: i64 = row
        .get(5)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie expiry: {}", e)))?;
    let secure: i64 = row.get(6).map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read cookie secure flag: {}", e))
    })?;
    let http_only: i64 = row.get(7).map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read cookie httponly flag: {}", e))
    })?;

    let cookie_value = if !value.is_empty() {
        value
    } else if !encrypted_value.is_empty() {
        match decryptor.decrypt(&encrypted_value) {
            Some(value) => value,
            None => return Ok(None),
        }
    } else {
        return Ok(None);
    };

    let expires = chromium_expires_to_unix_seconds(expires_utc);

    Ok(Some(Cookie {
        name,
        value: cookie_value,
        domain: host_key,
        path,
        secure: secure != 0,
        http_only: http_only != 0,
        expires,
    }))
}

fn read_encrypted_value(row: &Row<'_>) -> Result<Vec<u8>> {
    let value = row.get_ref(3).map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read cookie ciphertext: {}", e))
    })?;
    match value {
        rusqlite::types::ValueRef::Blob(bytes) => Ok(bytes.to_vec()),
        rusqlite::types::ValueRef::Text(text) => Ok(text.to_vec()),
        rusqlite::types::ValueRef::Null => Ok(Vec::new()),
        _ => Err(RurlError::BrowserCookie(
            "Unsupported cookie ciphertext type".to_string(),
        )),
    }
}

fn chromium_expires_to_unix_seconds(expires_utc: i64) -> Option<i64> {
    if expires_utc == 0 {
        return None;
    }
    let unix_seconds = (expires_utc / 1_000_000) - 11_644_473_600;
    if unix_seconds <= 0 {
        None
    } else {
        Some(unix_seconds)
    }
}

struct WindowsChromeCookieDecryptor {
    v10_key: Option<Vec<u8>>,
    meta_version: i64,
}

impl WindowsChromeCookieDecryptor {
    fn new(settings: &ChromiumSettings, meta_version: i64) -> Result<Self> {
        let v10_key = read_windows_v10_key(&settings.user_data_dir)?;
        Ok(Self {
            v10_key,
            meta_version,
        })
    }

    fn decrypt(&self, encrypted_value: &[u8]) -> Option<String> {
        if encrypted_value.len() < 3 {
            return None;
        }
        let (version, ciphertext) = encrypted_value.split_at(3);
        if version == WINDOWS_V10_PREFIX {
            let key = self.v10_key.as_ref()?;
            let plaintext = decrypt_aes_gcm(ciphertext, key).ok()?;
            decode_cookie_value(&plaintext, self.meta_version)
        } else {
            let plaintext = decrypt_windows_dpapi(encrypted_value).ok()?;
            decode_cookie_value(&plaintext, self.meta_version)
        }
    }
}

fn decode_cookie_value(value: &[u8], meta_version: i64) -> Option<String> {
    let trimmed = if meta_version >= 24 && value.len() > 32 {
        &value[32..]
    } else {
        value
    };
    String::from_utf8(trimmed.to_vec()).ok()
}

fn read_windows_v10_key(browser_root: &Path) -> Result<Option<Vec<u8>>> {
    let candidates = match find_files(browser_root, "Local State") {
        Ok(candidates) => candidates,
        Err(_) => return Ok(None),
    };
    let local_state_path = match newest_path(candidates) {
        Some(path) => path,
        None => return Ok(None),
    };
    let data = fs::read_to_string(&local_state_path).map_err(|e| {
        crate::browser::map_cookie_io_error(
            "Failed to read Local State",
            &local_state_path,
            e,
            Some("Close the browser or run without elevation."),
        )
    })?;
    let json: serde_json::Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let encrypted_key = json
        .get("os_crypt")
        .and_then(|value| value.get("encrypted_key"))
        .and_then(|value| value.as_str());
    let encrypted_key = match encrypted_key {
        Some(key) => key,
        None => return Ok(None),
    };
    let encrypted_bytes = match STANDARD.decode(encrypted_key) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    if !encrypted_bytes.starts_with(WINDOWS_DPAPI_PREFIX) {
        log::warn!("Invalid DPAPI prefix in Local State");
        return Ok(None);
    }
    Ok(decrypt_windows_dpapi(&encrypted_bytes[WINDOWS_DPAPI_PREFIX.len()..]).ok())
}

fn decrypt_aes_gcm(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(RurlError::BrowserCookie(
            "Invalid AES-GCM key length".to_string(),
        ));
    }
    if ciphertext.len() < AES_GCM_NONCE_LEN + AES_GCM_TAG_LEN {
        return Err(RurlError::BrowserCookie(
            "Invalid AES-GCM ciphertext length".to_string(),
        ));
    }
    let (nonce_bytes, payload) = ciphertext.split_at(AES_GCM_NONCE_LEN);
    let nonce_array: [u8; AES_GCM_NONCE_LEN] = nonce_bytes
        .try_into()
        .map_err(|_| RurlError::BrowserCookie("Invalid nonce length".to_string()))?;
    let nonce = aes_gcm::Nonce::from(nonce_array);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to create AES-GCM cipher: {}", e)))?;
    cipher
        .decrypt(&nonce, payload)
        .map_err(|_| RurlError::BrowserCookie("Failed to decrypt cookie".to_string()))
}

fn decrypt_windows_dpapi(ciphertext: &[u8]) -> Result<Vec<u8>> {
    unsafe {
        let mut in_blob = DATA_BLOB {
            cbData: ciphertext.len() as u32,
            pbData: ciphertext.as_ptr() as *mut u8,
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let result = CryptUnprotectData(
            &mut in_blob,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut out_blob,
        );
        if result == 0 {
            return Err(RurlError::BrowserCookie(
                "Failed to decrypt with DPAPI".to_string(),
            ));
        }

        let data = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        LocalFree(out_blob.pbData as *mut _);
        Ok(data)
    }
}
