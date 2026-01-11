use super::ChromiumBrowser;
use crate::browser::{Cookie, CookieStore};
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};
use aes::Aes128;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use dirs::{config_dir, home_dir};
use pbkdf2::pbkdf2_hmac;
use rusqlite::{Connection, Row};
use secret_service::blocking::SecretService;
use secret_service::EncryptionType;
use sha1::Sha1;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

use crate::utils::FileUtils;

const KEY_DERIVE_SALT: &[u8] = b"saltysalt";
const KEY_DERIVE_ITERATIONS: u32 = 1;
const KEY_LENGTH: usize = 16;
const AES_IV: &[u8; 16] = b"                ";
const LINUX_V10_PASSWORD: &[u8] = b"peanuts";

struct ChromiumSettings {
    user_data_dir: PathBuf,
    keyring_name: &'static str,
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
    fs::copy(&cookie_db, &temp_db)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to copy cookies DB: {}", e)))?;

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

    let decryptor =
        LinuxChromeCookieDecryptor::new(&settings, meta_version, config.keyring.as_deref())?;
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
    let config_home = config_dir().or_else(|| home_dir().map(|home| home.join(".config")));
    let config_home = config_home
        .ok_or_else(|| RurlError::Config("Cannot determine config directory".to_string()))?;
    let (relative_dir, keyring_name, supports_profiles) = match browser {
        ChromiumBrowser::Chrome => ("google-chrome", "Chrome", true),
        ChromiumBrowser::Edge => ("microsoft-edge", "Chromium", true),
        ChromiumBrowser::Brave => ("BraveSoftware/Brave-Browser", "Brave", true),
        ChromiumBrowser::Opera => ("opera", "Chromium", false),
        ChromiumBrowser::Vivaldi => ("vivaldi", "Chrome", true),
        ChromiumBrowser::Whale => ("naver-whale", "Whale", true),
    };
    Ok(ChromiumSettings {
        user_data_dir: config_home.join(relative_dir),
        keyring_name,
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

fn row_to_cookie(row: &Row<'_>, decryptor: &LinuxChromeCookieDecryptor) -> Result<Option<Cookie>> {
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

struct LinuxChromeCookieDecryptor {
    v10_key: [u8; KEY_LENGTH],
    empty_key: [u8; KEY_LENGTH],
    v11_key: Option<[u8; KEY_LENGTH]>,
    meta_version: i64,
}

impl LinuxChromeCookieDecryptor {
    fn new(settings: &ChromiumSettings, meta_version: i64, keyring: Option<&str>) -> Result<Self> {
        let v10_key = derive_key(LINUX_V10_PASSWORD);
        let empty_key = derive_key(b"");
        let password = get_linux_keyring_password(settings.keyring_name, keyring)?;
        let v11_key = password.map(|password| derive_key(&password));

        Ok(Self {
            v10_key,
            empty_key,
            v11_key,
            meta_version,
        })
    }

    fn decrypt(&self, encrypted_value: &[u8]) -> Option<String> {
        if encrypted_value.len() < 3 {
            return None;
        }
        let (version, ciphertext) = encrypted_value.split_at(3);
        if version == b"v10" {
            decrypt_aes_cbc_multi(
                ciphertext,
                [&self.v10_key, &self.empty_key],
                self.meta_version >= 24,
            )
        } else if version == b"v11" {
            let v11_key = self.v11_key.as_ref()?;
            decrypt_aes_cbc_multi(
                ciphertext,
                [v11_key, &self.empty_key],
                self.meta_version >= 24,
            )
        } else {
            log::warn!("Unknown Chrome cookie version: {:?}", version);
            None
        }
    }
}

fn derive_key(password: &[u8]) -> [u8; KEY_LENGTH] {
    let mut key = [0u8; KEY_LENGTH];
    pbkdf2_hmac::<Sha1>(password, KEY_DERIVE_SALT, KEY_DERIVE_ITERATIONS, &mut key);
    key
}

fn decrypt_aes_cbc_multi(
    ciphertext: &[u8],
    keys: [&[u8; KEY_LENGTH]; 2],
    hash_prefix: bool,
) -> Option<String> {
    for key in keys {
        let decrypted = decrypt_aes_cbc(ciphertext, key).ok()?;
        let trimmed = if hash_prefix && decrypted.len() > 32 {
            &decrypted[32..]
        } else {
            &decrypted[..]
        };
        if let Ok(value) = String::from_utf8(trimmed.to_vec()) {
            return Some(value);
        }
    }
    log::warn!("Failed to decrypt Chrome cookie: UTF-8 decode failed");
    None
}

fn decrypt_aes_cbc(ciphertext: &[u8], key: &[u8; KEY_LENGTH]) -> Result<Vec<u8>> {
    let mut buffer = ciphertext.to_vec();
    let decryptor = cbc::Decryptor::<Aes128>::new_from_slices(key, AES_IV)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to create AES decryptor: {}", e)))?;
    let plaintext = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| RurlError::BrowserCookie("Failed to decrypt cookie".to_string()))?;
    Ok(plaintext.to_vec())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxDesktopEnvironment {
    Other,
    Cinnamon,
    Deepin,
    Gnome,
    Kde3,
    Kde4,
    Kde5,
    Kde6,
    Pantheon,
    Ukui,
    Unity,
    Xfce,
    Lxqt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxKeyring {
    KWallet,
    KWallet5,
    KWallet6,
    GnomeKeyring,
    BasicText,
}

fn get_linux_desktop_environment() -> LinuxDesktopEnvironment {
    let xdg_current_desktop = env::var("XDG_CURRENT_DESKTOP").ok();
    let desktop_session = env::var("DESKTOP_SESSION").unwrap_or_default();

    if let Some(xdg_current_desktop) = xdg_current_desktop {
        for part in xdg_current_desktop.split(':').map(|part| part.trim()) {
            match part {
                "Unity" => {
                    if desktop_session.contains("gnome-fallback") {
                        return LinuxDesktopEnvironment::Gnome;
                    }
                    return LinuxDesktopEnvironment::Unity;
                }
                "Deepin" => return LinuxDesktopEnvironment::Deepin,
                "GNOME" => return LinuxDesktopEnvironment::Gnome,
                "X-Cinnamon" => return LinuxDesktopEnvironment::Cinnamon,
                "KDE" => {
                    let kde_version = env::var("KDE_SESSION_VERSION").ok();
                    return match kde_version.as_deref() {
                        Some("5") => LinuxDesktopEnvironment::Kde5,
                        Some("6") => LinuxDesktopEnvironment::Kde6,
                        Some("4") => LinuxDesktopEnvironment::Kde4,
                        _ => LinuxDesktopEnvironment::Kde4,
                    };
                }
                "Pantheon" => return LinuxDesktopEnvironment::Pantheon,
                "XFCE" => return LinuxDesktopEnvironment::Xfce,
                "UKUI" => return LinuxDesktopEnvironment::Ukui,
                "LXQt" => return LinuxDesktopEnvironment::Lxqt,
                _ => {}
            }
        }
    }

    match desktop_session.as_str() {
        "deepin" => return LinuxDesktopEnvironment::Deepin,
        "mate" | "gnome" => return LinuxDesktopEnvironment::Gnome,
        "kde4" | "kde-plasma" => return LinuxDesktopEnvironment::Kde4,
        "kde" => {
            if env::var("KDE_SESSION_VERSION").is_ok() {
                return LinuxDesktopEnvironment::Kde4;
            }
            return LinuxDesktopEnvironment::Kde3;
        }
        "ukui" => return LinuxDesktopEnvironment::Ukui,
        _ => {}
    }

    if desktop_session.contains("xfce") || desktop_session == "xubuntu" {
        return LinuxDesktopEnvironment::Xfce;
    }

    if env::var("GNOME_DESKTOP_SESSION_ID").is_ok() {
        return LinuxDesktopEnvironment::Gnome;
    }
    if env::var("KDE_FULL_SESSION").is_ok() {
        if env::var("KDE_SESSION_VERSION").is_ok() {
            return LinuxDesktopEnvironment::Kde4;
        }
        return LinuxDesktopEnvironment::Kde3;
    }

    LinuxDesktopEnvironment::Other
}

fn choose_linux_keyring() -> LinuxKeyring {
    let desktop_environment = get_linux_desktop_environment();
    match desktop_environment {
        LinuxDesktopEnvironment::Kde4 => LinuxKeyring::KWallet,
        LinuxDesktopEnvironment::Kde5 => LinuxKeyring::KWallet5,
        LinuxDesktopEnvironment::Kde6 => LinuxKeyring::KWallet6,
        LinuxDesktopEnvironment::Kde3
        | LinuxDesktopEnvironment::Lxqt
        | LinuxDesktopEnvironment::Other => LinuxKeyring::BasicText,
        _ => LinuxKeyring::GnomeKeyring,
    }
}

fn get_linux_keyring_password(
    browser_keyring_name: &str,
    keyring: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let keyring = if let Some(keyring) = keyring {
        parse_linux_keyring(keyring)?
    } else {
        choose_linux_keyring()
    };

    match keyring {
        LinuxKeyring::KWallet | LinuxKeyring::KWallet5 | LinuxKeyring::KWallet6 => {
            Ok(Some(get_kwallet_password(browser_keyring_name, keyring)))
        }
        LinuxKeyring::GnomeKeyring => Ok(Some(get_gnome_keyring_password(browser_keyring_name)?)),
        LinuxKeyring::BasicText => Ok(None),
    }
}

fn parse_linux_keyring(value: &str) -> Result<LinuxKeyring> {
    match value.to_lowercase().as_str() {
        "kwallet" => Ok(LinuxKeyring::KWallet),
        "kwallet5" => Ok(LinuxKeyring::KWallet5),
        "kwallet6" => Ok(LinuxKeyring::KWallet6),
        "gnome" | "gnomekeyring" => Ok(LinuxKeyring::GnomeKeyring),
        "basic" | "basictext" => Ok(LinuxKeyring::BasicText),
        _ => Err(RurlError::Config(format!("Unsupported keyring: {}", value))),
    }
}

fn get_kwallet_password(browser_keyring_name: &str, keyring: LinuxKeyring) -> Vec<u8> {
    let network_wallet = get_kwallet_network_wallet(keyring);
    let output = Command::new("kwallet-query")
        .args([
            "--read-password",
            &format!("{} Safe Storage", browser_keyring_name),
            "--folder",
            &format!("{} Keys", browser_keyring_name),
            &network_wallet,
        ])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(err) => {
            log::warn!("kwallet-query command failed: {}", err);
            return Vec::new();
        }
    };

    if !output.status.success() {
        log::warn!(
            "kwallet-query failed with status {}",
            output.status.code().unwrap_or(-1)
        );
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.to_lowercase().starts_with("failed to read") {
        log::debug!("Failed to read password from kwallet");
        return Vec::new();
    }
    stdout.trim_end_matches('\n').as_bytes().to_vec()
}

fn get_kwallet_network_wallet(keyring: LinuxKeyring) -> String {
    let default_wallet = "kdewallet".to_string();
    let (service_name, wallet_path) = match keyring {
        LinuxKeyring::KWallet => ("org.kde.kwalletd", "/modules/kwalletd"),
        LinuxKeyring::KWallet5 => ("org.kde.kwalletd5", "/modules/kwalletd5"),
        LinuxKeyring::KWallet6 => ("org.kde.kwalletd6", "/modules/kwalletd6"),
        _ => return default_wallet,
    };

    let output = Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply=literal",
            &format!("--dest={}", service_name),
            wallet_path,
            "org.kde.KWallet.networkWallet",
        ])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(err) => {
            log::warn!("dbus-send failed: {}", err);
            return default_wallet;
        }
    };

    if !output.status.success() {
        log::warn!(
            "dbus-send failed with status {}",
            output.status.code().unwrap_or(-1)
        );
        return default_wallet;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(start) = line.find("string \"") {
            let rest = &line[start + "string \"".len()..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    default_wallet
}

fn get_gnome_keyring_password(browser_keyring_name: &str) -> Result<Vec<u8>> {
    let service = match SecretService::connect(EncryptionType::Dh) {
        Ok(service) => service,
        Err(err) => {
            log::warn!("Failed to connect to secret service: {}", err);
            return Ok(Vec::new());
        }
    };

    let collection = service
        .get_default_collection()
        .or_else(|_| service.get_any_collection());
    let collection = match collection {
        Ok(collection) => collection,
        Err(err) => {
            log::warn!("Failed to read keyring collection: {}", err);
            return Ok(Vec::new());
        }
    };

    let items = match collection.get_all_items() {
        Ok(items) => items,
        Err(err) => {
            log::warn!("Failed to read keyring items: {}", err);
            return Ok(Vec::new());
        }
    };

    let label = format!("{} Safe Storage", browser_keyring_name);
    for item in items {
        let item_label = item.get_label().unwrap_or_default();
        if item_label == label {
            if item.is_locked().unwrap_or(false) {
                if let Err(err) = item.unlock() {
                    log::warn!("Failed to unlock keyring item: {}", err);
                }
            }
            match item.get_secret() {
                Ok(secret) => return Ok(secret),
                Err(err) => {
                    log::warn!("Failed to read keyring secret: {}", err);
                    return Ok(Vec::new());
                }
            }
        }
    }

    log::warn!("Failed to read from keyring");
    Ok(Vec::new())
}
