//! Safari browser cookie extraction (macOS only)

use crate::browser::CookieStore;
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};

/// Extract cookies from Safari browser
pub async fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    #[cfg(target_os = "macos")]
    {
        macos::extract_cookies(config)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = config;
        Err(RurlError::Unsupported(
            "Safari is only available on macOS".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use crate::browser::Cookie;
    use std::fs;
    use std::path::PathBuf;

    use crate::utils::FileUtils;

    const SAFARI_COOKIE_MAGIC: &[u8; 4] = b"cook";
    const SAFARI_PAGE_MAGIC: &[u8; 4] = b"\x00\x00\x01\x00";
    const MAC_EPOCH_OFFSET: i64 = 978307200;

    pub fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
        let cookies_path = safari_cookie_path(config.profile.as_deref())?;
        let data = fs::read(&cookies_path).map_err(|e| {
            RurlError::BrowserCookie(format!("Failed to read Safari cookies: {}", e))
        })?;

        let mut store: CookieStore = CookieStore::new();
        parse_safari_cookies(&data, &mut store)?;

        if store.is_empty() {
            return Err(RurlError::BrowserCookie(
                "No Safari cookies could be extracted".to_string(),
            ));
        }

        Ok(store)
    }

    fn safari_cookie_path(profile: Option<&str>) -> Result<PathBuf> {
        if let Some(profile) = profile {
            let expanded = FileUtils::expand_path(profile)?;
            if expanded.is_file() {
                return Ok(expanded);
            }
            return Err(RurlError::FileNotFound(
                "Custom Safari cookies path not found".to_string(),
            ));
        }

        let primary = expand_home("~/Library/Cookies/Cookies.binarycookies")?;
        if primary.is_file() {
            return Ok(primary);
        }
        let secondary = expand_home(
            "~/Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies",
        )?;
        if secondary.is_file() {
            return Ok(secondary);
        }

        Err(RurlError::FileNotFound(
            "Safari cookies database not found".to_string(),
        ))
    }

    fn expand_home(path: &str) -> Result<PathBuf> {
        FileUtils::expand_path(path)
    }

    fn parse_safari_cookies(data: &[u8], store: &mut CookieStore) -> Result<()> {
        let mut parser = DataParser::new(data);
        parser.expect_bytes(SAFARI_COOKIE_MAGIC, "database signature")?;
        let page_count = parser.read_u32_be()? as usize;
        let mut page_sizes = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            page_sizes.push(parser.read_u32_be()? as usize);
        }

        let mut body_cursor = parser.cursor();
        for page_size in page_sizes {
            let page_end = body_cursor + page_size;
            let page = data
                .get(body_cursor..page_end)
                .ok_or_else(|| RurlError::BrowserCookie("Invalid Safari page size".to_string()))?;
            parse_safari_page(page, store)?;
            body_cursor = page_end;
        }

        Ok(())
    }

    fn parse_safari_page(data: &[u8], store: &mut CookieStore) -> Result<()> {
        let mut parser = DataParser::new(data);
        parser.expect_bytes(SAFARI_PAGE_MAGIC, "page signature")?;
        let record_count = parser.read_u32_le()? as usize;
        let mut record_offsets = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            record_offsets.push(parser.read_u32_le()? as usize);
        }

        if record_count == 0 {
            return Ok(());
        }

        for offset in record_offsets {
            if offset >= data.len() {
                continue;
            }
            let record = &data[offset..];
            if let Some(cookie) = parse_safari_record(record)? {
                store.entry(cookie.domain.clone()).or_default().push(cookie);
            }
        }

        Ok(())
    }

    fn parse_safari_record(data: &[u8]) -> Result<Option<Cookie>> {
        let mut parser = DataParser::new(data);
        let _record_size = parser.read_u32_le()? as usize;
        parser.skip(4)?;
        let flags = parser.read_u32_le()?;
        let is_secure = flags & 0x0001 != 0;
        parser.skip(4)?;
        let domain_offset = parser.read_u32_le()? as usize;
        let name_offset = parser.read_u32_le()? as usize;
        let path_offset = parser.read_u32_le()? as usize;
        let value_offset = parser.read_u32_le()? as usize;
        parser.skip(8)?;
        let expiration = parser.read_f64_le()?;
        let _creation = parser.read_f64_le()?;

        let domain = read_cstring_at(data, domain_offset)?;
        let name = read_cstring_at(data, name_offset)?;
        let path = read_cstring_at(data, path_offset)?;
        let value = read_cstring_at(data, value_offset)?;

        if domain.is_empty() || name.is_empty() {
            return Ok(None);
        }

        let expires = Some(mac_absolute_to_unix(expiration));

        Ok(Some(Cookie {
            name,
            value,
            domain,
            path,
            secure: is_secure,
            http_only: false,
            expires,
        }))
    }

    fn read_cstring_at(data: &[u8], offset: usize) -> Result<String> {
        if offset >= data.len() {
            return Err(RurlError::BrowserCookie(
                "Safari cookie offset out of bounds".to_string(),
            ));
        }
        let slice = &data[offset..];
        let end = slice.iter().position(|byte| *byte == 0).ok_or_else(|| {
            RurlError::BrowserCookie("Safari cookie string not terminated".to_string())
        })?;
        let string = std::str::from_utf8(&slice[..end]).map_err(|_| {
            RurlError::BrowserCookie("Safari cookie string decode failed".to_string())
        })?;
        Ok(string.to_string())
    }

    fn mac_absolute_to_unix(timestamp: f64) -> i64 {
        MAC_EPOCH_OFFSET + timestamp as i64
    }

    struct DataParser<'a> {
        data: &'a [u8],
        cursor: usize,
    }

    impl<'a> DataParser<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self { data, cursor: 0 }
        }

        fn cursor(&self) -> usize {
            self.cursor
        }

        fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
            let end = self.cursor + len;
            let slice = self
                .data
                .get(self.cursor..end)
                .ok_or_else(|| RurlError::BrowserCookie("Safari cookies truncated".to_string()))?;
            self.cursor = end;
            Ok(slice)
        }

        fn expect_bytes(&mut self, expected: &[u8], label: &str) -> Result<()> {
            let actual = self.read_bytes(expected.len())?;
            if actual != expected {
                return Err(RurlError::BrowserCookie(format!(
                    "Safari cookies invalid {}",
                    label
                )));
            }
            Ok(())
        }

        fn read_u32_be(&mut self) -> Result<u32> {
            let bytes = self.read_bytes(4)?;
            Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
        }

        fn read_u32_le(&mut self) -> Result<u32> {
            let bytes = self.read_bytes(4)?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        }

        fn read_f64_le(&mut self) -> Result<f64> {
            let bytes = self.read_bytes(8)?;
            let bits = u64::from_le_bytes(bytes.try_into().unwrap());
            Ok(f64::from_bits(bits))
        }

        fn skip(&mut self, len: usize) -> Result<()> {
            self.read_bytes(len)?;
            Ok(())
        }
    }
}
