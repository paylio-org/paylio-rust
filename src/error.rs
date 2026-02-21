use std::fmt;

/// Error types returned by the Paylio SDK.
///
/// Each variant corresponds to a specific category of API error,
/// mapped from HTTP status codes.
#[derive(Debug, Clone)]
pub enum PaylioError {
    /// Invalid or missing API key (HTTP 401).
    Authentication {
        message: String,
        http_status: Option<u16>,
        http_body: Option<String>,
        code: Option<String>,
    },
    /// Bad request parameters (HTTP 400).
    InvalidRequest {
        message: String,
        http_status: Option<u16>,
        http_body: Option<String>,
        code: Option<String>,
    },
    /// Resource not found (HTTP 404).
    NotFound {
        message: String,
        http_status: Option<u16>,
        http_body: Option<String>,
        code: Option<String>,
    },
    /// Rate limit exceeded (HTTP 429).
    RateLimit {
        message: String,
        http_status: Option<u16>,
        http_body: Option<String>,
        code: Option<String>,
    },
    /// Generic API error (HTTP 5xx or unrecognized status).
    Api {
        message: String,
        http_status: Option<u16>,
        http_body: Option<String>,
        code: Option<String>,
    },
    /// Network or connection error (timeout, DNS failure, etc.).
    ApiConnection { message: String },
}

impl fmt::Display for PaylioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for PaylioError {}

impl PaylioError {
    /// Returns the error message.
    pub fn message(&self) -> &str {
        match self {
            Self::Authentication { message, .. }
            | Self::InvalidRequest { message, .. }
            | Self::NotFound { message, .. }
            | Self::RateLimit { message, .. }
            | Self::Api { message, .. }
            | Self::ApiConnection { message } => message,
        }
    }

    /// Returns the HTTP status code, if available.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::Authentication { http_status, .. }
            | Self::InvalidRequest { http_status, .. }
            | Self::NotFound { http_status, .. }
            | Self::RateLimit { http_status, .. }
            | Self::Api { http_status, .. } => *http_status,
            Self::ApiConnection { .. } => None,
        }
    }

    /// Returns the raw HTTP response body, if available.
    pub fn http_body(&self) -> Option<&str> {
        match self {
            Self::Authentication { http_body, .. }
            | Self::InvalidRequest { http_body, .. }
            | Self::NotFound { http_body, .. }
            | Self::RateLimit { http_body, .. }
            | Self::Api { http_body, .. } => http_body.as_deref(),
            Self::ApiConnection { .. } => None,
        }
    }

    /// Returns the error code from the API response, if available.
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Authentication { code, .. }
            | Self::InvalidRequest { code, .. }
            | Self::NotFound { code, .. }
            | Self::RateLimit { code, .. }
            | Self::Api { code, .. } => code.as_deref(),
            Self::ApiConnection { .. } => None,
        }
    }

    pub fn is_authentication(&self) -> bool {
        matches!(self, Self::Authentication { .. })
    }

    pub fn is_invalid_request(&self) -> bool {
        matches!(self, Self::InvalidRequest { .. })
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }

    pub fn is_api_connection(&self) -> bool {
        matches!(self, Self::ApiConnection { .. })
    }
}

/// Maps an HTTP status code to the appropriate error variant.
pub(crate) fn error_for_status(
    status: u16,
    message: String,
    http_body: Option<String>,
    code: Option<String>,
) -> PaylioError {
    let http_status = Some(status);
    match status {
        401 => PaylioError::Authentication {
            message,
            http_status,
            http_body,
            code,
        },
        400 => PaylioError::InvalidRequest {
            message,
            http_status,
            http_body,
            code,
        },
        404 => PaylioError::NotFound {
            message,
            http_status,
            http_body,
            code,
        },
        429 => PaylioError::RateLimit {
            message,
            http_status,
            http_body,
            code,
        },
        _ => PaylioError::Api {
            message,
            http_status,
            http_body,
            code,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_error_display() {
        let err = PaylioError::Authentication {
            message: "Invalid API key".into(),
            http_status: Some(401),
            http_body: Some("{}".into()),
            code: Some("auth_error".into()),
        };
        assert_eq!(err.to_string(), "Invalid API key");
    }

    #[test]
    fn test_invalid_request_error_display() {
        let err = PaylioError::InvalidRequest {
            message: "Bad request".into(),
            http_status: Some(400),
            http_body: None,
            code: None,
        };
        assert_eq!(err.to_string(), "Bad request");
    }

    #[test]
    fn test_not_found_error_display() {
        let err = PaylioError::NotFound {
            message: "Not found".into(),
            http_status: Some(404),
            http_body: None,
            code: None,
        };
        assert_eq!(err.to_string(), "Not found");
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = PaylioError::RateLimit {
            message: "Too many requests".into(),
            http_status: Some(429),
            http_body: None,
            code: None,
        };
        assert_eq!(err.to_string(), "Too many requests");
    }

    #[test]
    fn test_api_error_display() {
        let err = PaylioError::Api {
            message: "Server error".into(),
            http_status: Some(500),
            http_body: None,
            code: None,
        };
        assert_eq!(err.to_string(), "Server error");
    }

    #[test]
    fn test_api_connection_error_display() {
        let err = PaylioError::ApiConnection {
            message: "Connection refused".into(),
        };
        assert_eq!(err.to_string(), "Connection refused");
    }

    #[test]
    fn test_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(PaylioError::Api {
            message: "test".into(),
            http_status: None,
            http_body: None,
            code: None,
        });
        assert_eq!(err.to_string(), "test");
    }

    #[test]
    fn test_error_is_debug() {
        let err = PaylioError::Authentication {
            message: "test".into(),
            http_status: Some(401),
            http_body: None,
            code: None,
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("Authentication"));
    }

    #[test]
    fn test_message_accessor() {
        let err = PaylioError::Api {
            message: "hello".into(),
            http_status: Some(500),
            http_body: Some("body".into()),
            code: Some("err_code".into()),
        };
        assert_eq!(err.message(), "hello");
    }

    #[test]
    fn test_http_status_accessor() {
        let err = PaylioError::NotFound {
            message: "gone".into(),
            http_status: Some(404),
            http_body: None,
            code: None,
        };
        assert_eq!(err.http_status(), Some(404));
    }

    #[test]
    fn test_http_status_none_for_connection_error() {
        let err = PaylioError::ApiConnection {
            message: "timeout".into(),
        };
        assert_eq!(err.http_status(), None);
    }

    #[test]
    fn test_http_body_accessor() {
        let err = PaylioError::Api {
            message: "err".into(),
            http_status: Some(500),
            http_body: Some("{\"error\":\"oops\"}".into()),
            code: None,
        };
        assert_eq!(err.http_body(), Some("{\"error\":\"oops\"}"));
    }

    #[test]
    fn test_code_accessor() {
        let err = PaylioError::InvalidRequest {
            message: "bad".into(),
            http_status: Some(400),
            http_body: None,
            code: Some("invalid_param".into()),
        };
        assert_eq!(err.code(), Some("invalid_param"));
    }

    #[test]
    fn test_code_none_for_connection_error() {
        let err = PaylioError::ApiConnection {
            message: "timeout".into(),
        };
        assert_eq!(err.code(), None);
    }

    #[test]
    fn test_is_authentication() {
        let auth = PaylioError::Authentication {
            message: "bad key".into(),
            http_status: Some(401),
            http_body: None,
            code: None,
        };
        assert!(auth.is_authentication());
        assert!(!auth.is_not_found());
        assert!(!auth.is_invalid_request());
        assert!(!auth.is_rate_limit());
        assert!(!auth.is_api());
        assert!(!auth.is_api_connection());
    }

    #[test]
    fn test_is_invalid_request() {
        let err = PaylioError::InvalidRequest {
            message: "bad".into(),
            http_status: Some(400),
            http_body: None,
            code: None,
        };
        assert!(err.is_invalid_request());
        assert!(!err.is_authentication());
    }

    #[test]
    fn test_is_not_found() {
        let err = PaylioError::NotFound {
            message: "gone".into(),
            http_status: Some(404),
            http_body: None,
            code: None,
        };
        assert!(err.is_not_found());
        assert!(!err.is_api());
    }

    #[test]
    fn test_is_rate_limit() {
        let err = PaylioError::RateLimit {
            message: "slow down".into(),
            http_status: Some(429),
            http_body: None,
            code: None,
        };
        assert!(err.is_rate_limit());
    }

    #[test]
    fn test_is_api() {
        let err = PaylioError::Api {
            message: "oops".into(),
            http_status: Some(500),
            http_body: None,
            code: None,
        };
        assert!(err.is_api());
    }

    #[test]
    fn test_is_api_connection() {
        let err = PaylioError::ApiConnection {
            message: "refused".into(),
        };
        assert!(err.is_api_connection());
        assert!(!err.is_authentication());
    }

    #[test]
    fn test_error_for_status_401() {
        let err = error_for_status(401, "auth failed".into(), None, None);
        assert!(err.is_authentication());
        assert_eq!(err.http_status(), Some(401));
    }

    #[test]
    fn test_error_for_status_400() {
        let err = error_for_status(
            400,
            "bad request".into(),
            Some("body".into()),
            Some("code".into()),
        );
        assert!(err.is_invalid_request());
        assert_eq!(err.http_status(), Some(400));
        assert_eq!(err.http_body(), Some("body"));
        assert_eq!(err.code(), Some("code"));
    }

    #[test]
    fn test_error_for_status_404() {
        let err = error_for_status(404, "not found".into(), None, None);
        assert!(err.is_not_found());
    }

    #[test]
    fn test_error_for_status_429() {
        let err = error_for_status(429, "rate limited".into(), None, None);
        assert!(err.is_rate_limit());
    }

    #[test]
    fn test_error_for_status_500() {
        let err = error_for_status(500, "server error".into(), None, None);
        assert!(err.is_api());
        assert_eq!(err.http_status(), Some(500));
    }

    #[test]
    fn test_error_for_status_502() {
        let err = error_for_status(502, "bad gateway".into(), None, None);
        assert!(err.is_api());
        assert_eq!(err.http_status(), Some(502));
    }

    #[test]
    fn test_error_clone() {
        let err = PaylioError::Api {
            message: "test".into(),
            http_status: Some(500),
            http_body: Some("body".into()),
            code: Some("err".into()),
        };
        let cloned = err.clone();
        assert_eq!(cloned.message(), "test");
        assert_eq!(cloned.http_status(), Some(500));
    }

    #[test]
    fn test_http_body_none_for_connection_error() {
        let err = PaylioError::ApiConnection {
            message: "timeout".into(),
        };
        assert_eq!(err.http_body(), None);
    }

    #[test]
    fn test_rate_limit_http_status() {
        let err = PaylioError::RateLimit {
            message: "slow".into(),
            http_status: Some(429),
            http_body: Some("body".into()),
            code: Some("rate_limit".into()),
        };
        assert_eq!(err.http_status(), Some(429));
        assert_eq!(err.http_body(), Some("body"));
        assert_eq!(err.code(), Some("rate_limit"));
    }

    #[test]
    fn test_not_found_accessors() {
        let err = PaylioError::NotFound {
            message: "missing".into(),
            http_status: Some(404),
            http_body: Some("{\"error\":\"not found\"}".into()),
            code: Some("not_found".into()),
        };
        assert_eq!(err.http_body(), Some("{\"error\":\"not found\"}"));
        assert_eq!(err.code(), Some("not_found"));
    }

    #[test]
    fn test_authentication_accessors() {
        let err = PaylioError::Authentication {
            message: "bad key".into(),
            http_status: Some(401),
            http_body: Some("auth body".into()),
            code: Some("auth_err".into()),
        };
        assert_eq!(err.http_body(), Some("auth body"));
        assert_eq!(err.code(), Some("auth_err"));
    }
}
