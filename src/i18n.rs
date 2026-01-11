use crate::error::RurlError;
use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{static_loader, Loader};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

static_loader! {
    static LOCALES = {
        locales: "locales",
        fallback_language: "en-US",
    };
}

pub fn localize_error(err: &RurlError) -> String {
    let langid = resolve_language();
    match err {
        RurlError::InvalidUrl(detail) => message_with_detail(&langid, "error-invalid-url", detail),
        RurlError::Http(detail) => message_with_detail(&langid, "error-http", &detail.to_string()),
        RurlError::Json(detail) => message_with_detail(&langid, "error-json", &detail.to_string()),
        RurlError::Ssl(detail) => message_with_detail(&langid, "error-ssl", detail),
        RurlError::Proxy(detail) => message_with_detail(&langid, "error-proxy", detail),
        RurlError::Auth(detail) => message_with_detail(&langid, "error-auth", detail),
        RurlError::Config(detail) => message_with_detail(&langid, "error-config", detail),
        RurlError::Timeout => LOCALES.lookup(&langid, "error-timeout"),
        RurlError::RedirectLimitExceeded(limit) => {
            let mut args = HashMap::new();
            args.insert("limit", FluentValue::from(*limit));
            LOCALES.lookup_with_args(&langid, "error-redirect-limit", &args)
        }
        RurlError::PermissionDenied(detail) => {
            message_with_detail(&langid, "error-permission-denied", detail)
        }
        RurlError::FileNotFound(detail) => {
            message_with_detail(&langid, "error-file-not-found", detail)
        }
        RurlError::Unsupported(detail) => message_with_detail(&langid, "error-unsupported", detail),
        RurlError::Io(detail) => message_with_detail(&langid, "error-io", &detail.to_string()),
        RurlError::BrowserCookie(detail) => {
            message_with_detail(&langid, "error-browser-cookie", detail)
        }
    }
}

fn message_with_detail(langid: &LanguageIdentifier, key: &str, detail: &str) -> String {
    let mut args = HashMap::new();
    args.insert("detail", FluentValue::from(detail));
    LOCALES.lookup_with_args(langid, key, &args)
}

fn resolve_language() -> LanguageIdentifier {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if let Some(lang) = normalize_lang(value) {
                if let Ok(langid) = lang.parse::<LanguageIdentifier>() {
                    return langid;
                }
            }
        }
    }
    "en-US".parse().expect("valid fallback language")
}

fn normalize_lang(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let value = value.split('.').next().unwrap_or(value);
    let value = value.replace('_', "-");
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::{localize_error, normalize_lang};
    use crate::error::RurlError;

    #[test]
    fn normalize_lang_trims_and_normalizes() {
        assert_eq!(
            normalize_lang("en_US.UTF-8".to_string()),
            Some("en-US".to_string())
        );
        assert_eq!(normalize_lang("".to_string()), None);
    }

    #[test]
    fn localize_error_includes_detail() {
        let err = RurlError::InvalidUrl("detail".to_string());
        let message = localize_error(&err);
        assert!(message.contains("detail"));
    }
}
