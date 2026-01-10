//! Chrome/Chromium browser cookie extraction

use crate::browser::CookieStore;
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};

/// Supported Chromium-based browsers on macOS.
#[derive(Debug, Clone, Copy)]
pub enum ChromiumBrowser {
    Chrome,
    Edge,
    Brave,
    Opera,
    Vivaldi,
    Whale,
}

/// Extract cookies from Chrome browser
pub async fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    extract_chromium_cookies(ChromiumBrowser::Chrome, config)
}

pub fn extract_chromium_cookies(
    browser: ChromiumBrowser,
    config: &BrowserCookieConfig,
) -> Result<CookieStore> {
    #[cfg(target_os = "macos")]
    {
        macos::extract_chromium_cookies(browser, config)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (browser, config);
        Err(RurlError::Unsupported(
            "Chromium cookie extraction is only implemented for macOS".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use crate::browser::Cookie;
    use aes::Aes128;
    use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
    use dirs::home_dir;
    use pbkdf2::pbkdf2_hmac;
    use rusqlite::{Connection, Row};
    use security_framework::passwords::get_generic_password;
    use sha1::Sha1;
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    use crate::utils::FileUtils;

    const KEY_DERIVE_SALT: &[u8] = b"saltysalt";
    const KEY_DERIVE_ITERATIONS: u32 = 1003;
    const KEY_LENGTH: usize = 16;
    const AES_IV: &[u8; 16] = b"                ";

    struct ChromiumSettings {
        user_data_dir: PathBuf,
        keychain_account: &'static str,
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

        let decryptor = MacChromeCookieDecryptor::new(&settings, meta_version)?;
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

        let mut stmt = conn.prepare(&query).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to prepare cookie query: {}", e))
        })?;
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
        let home = home_dir()
            .ok_or_else(|| RurlError::Config("Cannot determine home directory".to_string()))?;
        let app_support = home.join("Library/Application Support");
        let (relative_dir, keychain_account, supports_profiles) = match browser {
            ChromiumBrowser::Chrome => ("Google/Chrome", "Chrome", true),
            ChromiumBrowser::Edge => ("Microsoft Edge", "Microsoft Edge", true),
            ChromiumBrowser::Brave => ("BraveSoftware/Brave-Browser", "Brave", true),
            ChromiumBrowser::Opera => ("com.operasoftware.Opera", "Opera", false),
            ChromiumBrowser::Vivaldi => ("Vivaldi", "Vivaldi", true),
            ChromiumBrowser::Whale => ("Naver/Whale", "Whale", true),
        };
        Ok(ChromiumSettings {
            user_data_dir: app_support.join(relative_dir),
            keychain_account,
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
        newest
            .ok_or_else(|| RurlError::FileNotFound("Chrome cookies database not found".to_string()))
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
        let mut stmt = conn.prepare("PRAGMA table_info(cookies)").map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e))
        })?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| {
                RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e))
            })?;
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
        decryptor: &MacChromeCookieDecryptor,
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
        let encrypted_value: Vec<u8> = row.get(3).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read cookie ciphertext: {}", e))
        })?;
        let path: String = row
            .get(4)
            .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie path: {}", e)))?;
        let expires_utc: i64 = row.get(5).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read cookie expiry: {}", e))
        })?;
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

        let expires = if expires_utc == 0 {
            None
        } else {
            Some(expires_utc)
        };

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

    struct MacChromeCookieDecryptor {
        key: Option<[u8; KEY_LENGTH]>,
        meta_version: i64,
    }

    impl MacChromeCookieDecryptor {
        fn new(settings: &ChromiumSettings, meta_version: i64) -> Result<Self> {
            let service = format!("{} Safe Storage", settings.keychain_account);
            let password = match get_generic_password(&service, settings.keychain_account) {
                Ok(password) => Some(password),
                Err(err) => {
                    log::warn!(
                        "Failed to read keychain password for {}: {}",
                        settings.keychain_account,
                        err
                    );
                    None
                }
            };

            let key = password.map(|pass| derive_key(&pass));
            Ok(Self { key, meta_version })
        }

        fn decrypt(&self, encrypted_value: &[u8]) -> Option<String> {
            if encrypted_value.len() < 3 {
                return None;
            }
            let (version, ciphertext) = encrypted_value.split_at(3);
            if version == b"v10" {
                let key = self.key.as_ref()?;
                let decrypted = decrypt_aes_cbc(ciphertext, key).ok()?;
                let trimmed = if self.meta_version >= 24 && decrypted.len() > 32 {
                    &decrypted[32..]
                } else {
                    &decrypted[..]
                };
                String::from_utf8(trimmed.to_vec()).ok()
            } else {
                String::from_utf8(encrypted_value.to_vec()).ok()
            }
        }
    }

    fn derive_key(password: &[u8]) -> [u8; KEY_LENGTH] {
        let mut key = [0u8; KEY_LENGTH];
        pbkdf2_hmac::<Sha1>(password, KEY_DERIVE_SALT, KEY_DERIVE_ITERATIONS, &mut key);
        key
    }

    fn decrypt_aes_cbc(ciphertext: &[u8], key: &[u8; KEY_LENGTH]) -> Result<Vec<u8>> {
        let mut buffer = ciphertext.to_vec();
        let decryptor = cbc::Decryptor::<Aes128>::new_from_slices(key, AES_IV).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to create AES decryptor: {}", e))
        })?;
        let plaintext = decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|_| RurlError::BrowserCookie("Failed to decrypt cookie".to_string()))?;
        Ok(plaintext.to_vec())
    }
}
