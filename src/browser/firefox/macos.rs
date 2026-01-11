use crate::browser::{Cookie, CookieStore};
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};
use rusqlite::{Connection, Row};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

use crate::utils::FileUtils;

const MAX_SUPPORTED_DB_SCHEMA_VERSION: i64 = 17;

pub fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    let search_roots = firefox_search_roots(config.profile.as_deref())?;
    let cookie_db = newest_path(find_cookie_dbs(&search_roots))
        .ok_or_else(|| RurlError::FileNotFound("Firefox cookies database not found".to_string()))?;

    let temp_dir = tempdir()
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to create temp dir: {}", e)))?;
    let temp_db = temp_dir.path().join("firefox-cookies.sqlite");
    fs::copy(&cookie_db, &temp_db).map_err(|e| {
        crate::browser::map_cookie_io_error("Failed to copy cookies DB", &cookie_db, e, None)
    })?;

    let conn = Connection::open(&temp_db)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to open cookies DB: {}", e)))?;
    let schema_version = read_schema_version(&conn);
    if schema_version > MAX_SUPPORTED_DB_SCHEMA_VERSION {
        log::warn!(
            "Firefox cookie DB schema version {} may be unsupported",
            schema_version
        );
    }

    let (expiry_column, http_only_column, secure_column) = cookie_columns(&conn)?;
    let (container_id, container_mode) =
        resolve_container(&cookie_db, config.container.as_deref())?;

    let mut store: CookieStore = CookieStore::new();
    let query = match container_mode {
        ContainerMode::Specific => format!(
            "SELECT host, name, value, path, {}, {}, {} FROM moz_cookies WHERE originAttributes LIKE ? OR originAttributes LIKE ?",
            expiry_column, secure_column, http_only_column
        ),
        ContainerMode::NoneOnly => format!(
            "SELECT host, name, value, path, {}, {}, {} FROM moz_cookies WHERE NOT INSTR(originAttributes, 'userContextId=')",
            expiry_column, secure_column, http_only_column
        ),
        ContainerMode::Any => format!(
            "SELECT host, name, value, path, {}, {}, {} FROM moz_cookies",
            expiry_column, secure_column, http_only_column
        ),
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to prepare Firefox query: {}", e)))?;
    let mut rows = match container_mode {
        ContainerMode::Specific => {
            let id = container_id.ok_or_else(|| {
                RurlError::BrowserCookie("Firefox container id not resolved".to_string())
            })?;
            stmt.query([
                format!("%userContextId={}", id),
                format!("%userContextId={}&%", id),
            ])
            .map_err(|e| {
                RurlError::BrowserCookie(format!("Failed to query Firefox cookies: {}", e))
            })?
        }
        _ => stmt.query([]).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to query Firefox cookies: {}", e))
        })?,
    };

    while let Some(row) = rows.next().map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read Firefox cookie row: {}", e))
    })? {
        if let Some(cookie) = row_to_cookie(row, schema_version)? {
            store.entry(cookie.domain.clone()).or_default().push(cookie);
        }
    }

    if store.is_empty() {
        return Err(RurlError::BrowserCookie(
            "No Firefox cookies could be extracted".to_string(),
        ));
    }

    Ok(store)
}

fn firefox_search_roots(profile: Option<&str>) -> Result<Vec<PathBuf>> {
    let base = dirs::home_dir()
        .ok_or_else(|| RurlError::Config("Cannot determine home directory".to_string()))?
        .join("Library/Application Support/Firefox/Profiles");

    if let Some(profile) = profile {
        if is_path_like(profile) {
            let expanded = FileUtils::expand_path(profile)?;
            return Ok(vec![expanded]);
        }
        return Ok(vec![base.join(profile)]);
    }

    Ok(vec![base])
}

fn is_path_like(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.starts_with('~')
}

fn find_cookie_dbs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    for root in roots {
        if root.is_file() && root.ends_with("cookies.sqlite") {
            results.push(root.clone());
            continue;
        }
        if root.exists() {
            results.extend(find_files(root, "cookies.sqlite"));
        }
    }
    results
}

fn find_files(root: &Path, filename: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(filename) {
                matches.push(path);
            }
        }
    }
    matches
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

fn read_schema_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
        .unwrap_or(0)
}

fn cookie_columns(conn: &Connection) -> Result<(String, String, String)> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(moz_cookies)")
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e)))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e)))?;
    let mut columns = Vec::new();
    for row in rows {
        let name = row.map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read cookie schema: {}", e))
        })?;
        columns.push(name);
    }
    let expiry_column = if columns.iter().any(|c| c == "expiry") {
        "expiry"
    } else if columns.iter().any(|c| c == "expires") {
        "expires"
    } else {
        return Err(RurlError::BrowserCookie(
            "Firefox cookies table missing expiry column".to_string(),
        ));
    };
    let secure_column = if columns.iter().any(|c| c == "isSecure") {
        "isSecure"
    } else if columns.iter().any(|c| c == "is_secure") {
        "is_secure"
    } else {
        "isSecure"
    };
    let http_only_column = if columns.iter().any(|c| c == "isHttpOnly") {
        "isHttpOnly"
    } else if columns.iter().any(|c| c == "is_http_only") {
        "is_http_only"
    } else {
        "0"
    };
    Ok((
        expiry_column.to_string(),
        http_only_column.to_string(),
        secure_column.to_string(),
    ))
}

fn row_to_cookie(row: &Row<'_>, schema_version: i64) -> Result<Option<Cookie>> {
    let domain: String = row
        .get(0)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie host: {}", e)))?;
    let name: String = row
        .get(1)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie name: {}", e)))?;
    let value: String = row
        .get(2)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie value: {}", e)))?;
    let path: String = row
        .get(3)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie path: {}", e)))?;
    let expiry: Option<i64> = row
        .get(4)
        .map_err(|e| RurlError::BrowserCookie(format!("Failed to read cookie expiry: {}", e)))?;
    let secure: i64 = row.get(5).map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read cookie secure flag: {}", e))
    })?;
    let http_only: i64 = row.get(6).map_err(|e| {
        RurlError::BrowserCookie(format!("Failed to read cookie http-only flag: {}", e))
    })?;

    let expiry_seconds = expiry.map(|expiry| {
        if schema_version >= 16 {
            expiry / 1000
        } else {
            expiry
        }
    });
    let expires = match expiry_seconds {
        Some(seconds) if seconds > 0 => Some(seconds),
        _ => None,
    };

    Ok(Some(Cookie {
        name,
        value,
        domain,
        path,
        secure: secure != 0,
        http_only: http_only != 0,
        expires,
    }))
}

fn resolve_container(
    cookie_db: &Path,
    container: Option<&str>,
) -> Result<(Option<i64>, ContainerMode)> {
    let container = match container {
        Some(container) => container,
        None => return Ok((None, ContainerMode::Any)),
    };

    if container == "none" {
        return Ok((None, ContainerMode::NoneOnly));
    }

    let containers_path = cookie_db
        .parent()
        .map(|path| path.join("containers.json"))
        .ok_or_else(|| RurlError::BrowserCookie("Firefox profile path not found".to_string()))?;
    if !containers_path.is_file() {
        return Err(RurlError::FileNotFound(
            "Firefox containers.json not found".to_string(),
        ));
    }

    let data = fs::read_to_string(&containers_path).map_err(|e| {
        crate::browser::map_cookie_io_error(
            "Failed to read containers.json",
            &containers_path,
            e,
            None,
        )
    })?;
    let value: Value = serde_json::from_str(&data)?;
    let identities = value
        .get("identities")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for identity in identities.iter() {
        let name = identity.get("name").and_then(|v| v.as_str());
        let l10n_id = identity.get("l10nID").and_then(|v| v.as_str());
        let user_context = identity.get("userContextId").and_then(|v| v.as_i64());

        if name == Some(container) || l10n_matches(container, l10n_id) {
            if let Some(id) = user_context {
                return Ok((Some(id), ContainerMode::Specific));
            }
        }
    }

    Err(RurlError::BrowserCookie(format!(
        "Firefox container '{}' not found",
        container
    )))
}

fn l10n_matches(container: &str, l10n_id: Option<&str>) -> bool {
    let l10n_id = match l10n_id {
        Some(l10n_id) => l10n_id,
        None => return false,
    };
    let pattern = "userContext";
    if !l10n_id.starts_with(pattern) || !l10n_id.ends_with(".label") {
        return false;
    }
    let label = &l10n_id[pattern.len()..l10n_id.len() - ".label".len()];
    label == container
}

enum ContainerMode {
    Any,
    NoneOnly,
    Specific,
}

#[cfg(test)]
mod tests {
    use super::{is_path_like, l10n_matches};

    #[test]
    fn is_path_like_detects_paths() {
        assert!(is_path_like("~/Library"));
        assert!(is_path_like("C:\\Users\\user"));
        assert!(is_path_like("/tmp/file"));
        assert!(!is_path_like("Profile 1"));
    }

    #[test]
    fn l10n_matches_accepts_known_pattern() {
        assert!(l10n_matches("Personal", Some("userContextPersonal.label")));
        assert!(!l10n_matches("Work", Some("userContextPersonal.label")));
        assert!(!l10n_matches("Personal", None));
        assert!(!l10n_matches("Personal", Some("invalid")));
    }
}
