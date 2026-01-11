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
}
