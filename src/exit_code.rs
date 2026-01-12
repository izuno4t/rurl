use crate::error::RurlError;

pub fn exit_code_for_error(err: &RurlError) -> i32 {
    match err {
        RurlError::InvalidUrl(_) => 3,
        RurlError::Config(_) => 2,
        RurlError::Proxy(_) => 5,
        RurlError::Auth(_) => 94,
        RurlError::Timeout => 28,
        RurlError::RedirectLimitExceeded(_) => 47,
        RurlError::PermissionDenied(_) | RurlError::FileNotFound(_) => 37,
        RurlError::Ssl(message) => ssl_exit_code(message),
        RurlError::Io(_) => 23,
        RurlError::Json(_) => 26,
        RurlError::Unsupported(_) => 4,
        RurlError::Http(err) => http_exit_code(err),
        RurlError::BrowserCookie(_) => 43,
    }
}

fn http_exit_code(err: &reqwest::Error) -> i32 {
    if err.is_timeout() {
        return 28;
    }
    if err.is_connect() {
        return 7;
    }
    if err.is_request() {
        return 2;
    }
    43
}

fn ssl_exit_code(message: &str) -> i32 {
    let lower = message.to_ascii_lowercase();
    if lower.contains("ca certificate") {
        return 77;
    }
    if lower.contains("client certificate") {
        return 58;
    }
    35
}

#[cfg(test)]
mod tests {
    use super::exit_code_for_error;
    use crate::error::RurlError;

    #[test]
    fn exit_code_maps_invalid_url() {
        let err = RurlError::InvalidUrl("bad".to_string());
        assert_eq!(exit_code_for_error(&err), 3);
    }

    #[test]
    fn exit_code_maps_redirect_limit() {
        let err = RurlError::RedirectLimitExceeded(2);
        assert_eq!(exit_code_for_error(&err), 47);
    }

    #[test]
    fn exit_code_maps_ssl_variants() {
        let err = RurlError::Ssl("CA certificate failed".to_string());
        assert_eq!(exit_code_for_error(&err), 77);
        let err = RurlError::Ssl("Client certificate missing".to_string());
        assert_eq!(exit_code_for_error(&err), 58);
        let err = RurlError::Ssl("TLS handshake error".to_string());
        assert_eq!(exit_code_for_error(&err), 35);
    }

    #[test]
    fn exit_code_maps_auth_and_config() {
        let err = RurlError::Auth("bad".to_string());
        assert_eq!(exit_code_for_error(&err), 94);
        let err = RurlError::Config("bad".to_string());
        assert_eq!(exit_code_for_error(&err), 2);
    }

    #[test]
    fn exit_code_maps_permission_and_not_found() {
        let err = RurlError::PermissionDenied("no".to_string());
        assert_eq!(exit_code_for_error(&err), 37);
        let err = RurlError::FileNotFound("missing".to_string());
        assert_eq!(exit_code_for_error(&err), 37);
    }

    #[test]
    fn exit_code_maps_additional_variants() {
        assert_eq!(
            exit_code_for_error(&RurlError::Proxy("bad proxy".to_string())),
            5
        );
        assert_eq!(
            exit_code_for_error(&RurlError::BrowserCookie("cookie".to_string())),
            43
        );
        assert_eq!(
            exit_code_for_error(&RurlError::Json(serde_json::Error::io(
                std::io::Error::other("json")
            ))),
            26
        );
        assert_eq!(
            exit_code_for_error(&RurlError::Unsupported("nope".to_string())),
            4
        );
        assert_eq!(
            exit_code_for_error(&RurlError::Io(std::io::Error::from(
                std::io::ErrorKind::Other
            ))),
            23
        );
        assert_eq!(exit_code_for_error(&RurlError::Timeout), 28);
    }
}
