use std::collections::HashMap;
use std::time::Duration;

use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::error::{error_for_status, PaylioError};
use crate::version::VERSION;

/// Default base URL for the Paylio API.
pub const DEFAULT_BASE_URL: &str = "https://api.paylio.pro/flying/v1";

/// Default request timeout (30 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Options for an HTTP request.
pub(crate) struct RequestOptions {
    pub params: Option<HashMap<String, String>>,
    pub json_body: Option<serde_json::Value>,
}

/// Internal HTTP client for making API requests.
pub(crate) struct HttpClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(api_key: &str, base_url: &str, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    pub async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<T, PaylioError> {
        let mut url = format!("{}{}", self.base_url, path);

        // Append query parameters
        if let Some(opts) = opts {
            if let Some(params) = &opts.params {
                let query: Vec<String> =
                    params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                if !query.is_empty() {
                    url = format!("{}?{}", url, query.join("&"));
                }
            }
        }

        let mut builder = self
            .client
            .request(method, &url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", format!("paylio-rust/{}", VERSION));

        // Add JSON body if provided
        if let Some(opts) = opts {
            if let Some(body) = &opts.json_body {
                builder = builder.json(body);
            }
        }

        let response = builder
            .send()
            .await
            .map_err(|e| PaylioError::ApiConnection {
                message: e.to_string(),
            })?;

        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, PaylioError> {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| PaylioError::ApiConnection {
                message: format!("Failed to read response body: {}", e),
            })?;

        if (200..300).contains(&status) {
            // Success: deserialize response
            serde_json::from_str::<T>(&body).map_err(|_| PaylioError::Api {
                message: format!("Invalid JSON response: {}", body),
                http_status: Some(status),
                http_body: Some(body.clone()),
                code: None,
            })
        } else {
            // Error: parse error body
            let (message, code) = self.parse_error_body(&body);
            Err(error_for_status(status, message, Some(body), code))
        }
    }

    fn parse_error_body(&self, body: &str) -> (String, Option<String>) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            // Format 1: {"error": {"code": "...", "message": "..."}}
            if let Some(error_obj) = json.get("error") {
                if let Some(obj) = error_obj.as_object() {
                    let message = obj
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or(body)
                        .to_string();
                    let code = obj.get("code").and_then(|v| v.as_str()).map(String::from);
                    return (message, code);
                }
                // Format 2: {"error": "string"}
                if let Some(msg) = error_obj.as_str() {
                    return (msg.to_string(), None);
                }
            }

            // Format 3: {"detail": "string"}
            if let Some(detail) = json.get("detail").and_then(|v| v.as_str()) {
                return (detail.to_string(), None);
            }

            // Unrecognized JSON structure
            return (body.to_string(), None);
        }

        // Non-JSON body
        (body.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.paylio.pro/flying/v1");
        assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_sends_correct_headers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .and(header("X-API-Key", "sk_test_abc"))
            .and(header("Content-Type", "application/json"))
            .and(header("Accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test_abc", &server.uri(), DEFAULT_TIMEOUT);
        let result: serde_json::Value = client.request(Method::GET, "/test", None).await.unwrap();
        assert_eq!(result["ok"], true);
    }

    #[tokio::test]
    async fn test_user_agent_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .and(header("User-Agent", format!("paylio-rust/{}", VERSION)))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let result: serde_json::Value = client.request(Method::GET, "/test", None).await.unwrap();
        assert_eq!(result["ok"], true);
    }

    #[tokio::test]
    async fn test_success_returns_deserialized_json() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/subscription/user_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sub_1",
                "object": "subscription",
                "status": "active",
                "user_id": "user_1",
                "plan": {"slug": "pro", "name": "Pro", "interval": "month", "amount": 9.99, "currency": "usd"},
                "subscription_period": {"start": "2026-01-01", "end": "2026-02-01"},
                "cancel_at_period_end": false,
                "canceled_at": null,
                "provider": "stripe",
                "created_at": "2025-12-01"
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let sub: crate::resources::Subscription = client
            .request(Method::GET, "/subscription/user_1", None)
            .await
            .unwrap();
        assert_eq!(sub.id, "sub_1");
        assert_eq!(sub.status, "active");
    }

    #[tokio::test]
    async fn test_non_json_success_returns_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_api());
        assert_eq!(err.http_status(), Some(200));
        assert!(err.message().contains("not json"));
    }

    #[tokio::test]
    async fn test_error_status_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"error": "Invalid API key"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_bad", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_authentication());
        assert_eq!(err.http_status(), Some(401));
    }

    #[tokio::test]
    async fn test_error_status_400() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"error": "Bad request"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_invalid_request());
    }

    #[tokio::test]
    async fn test_error_status_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "Not found"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_not_found());
    }

    #[tokio::test]
    async fn test_error_status_429() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(serde_json::json!({"error": "Rate limited"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_rate_limit());
    }

    #[tokio::test]
    async fn test_error_status_500() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"error": "Internal server error"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_api());
        assert_eq!(err.http_status(), Some(500));
    }

    #[tokio::test]
    async fn test_error_format_v1_structured() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": "invalid_param",
                    "message": "field is required"
                }
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "field is required");
        assert_eq!(err.code(), Some("invalid_param"));
    }

    #[tokio::test]
    async fn test_error_format_legacy_string() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"error": "legacy error message"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert_eq!(err.message(), "legacy error message");
        assert_eq!(err.code(), None);
    }

    #[tokio::test]
    async fn test_error_format_fastapi_detail() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"detail": "fastapi error message"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert_eq!(err.message(), "fastapi error message");
    }

    #[tokio::test]
    async fn test_error_non_json_fallback() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(500).set_body_string("plain text error"))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_api());
        assert_eq!(err.message(), "plain text error");
    }

    #[tokio::test]
    async fn test_error_preserves_http_body() {
        let server = MockServer::start().await;
        let body = r#"{"error":"test error"}"#;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(500).set_body_string(body))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.http_body().unwrap().contains("test error"));
    }

    #[tokio::test]
    async fn test_get_with_query_params() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/user_1/subscriptions"))
            .and(query_param("page", "2"))
            .and(query_param("page_size", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "total": 0,
                "page": 2,
                "page_size": 5,
                "total_pages": 0
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let mut params = HashMap::new();
        params.insert("page".to_string(), "2".to_string());
        params.insert("page_size".to_string(), "5".to_string());
        let opts = RequestOptions {
            params: Some(params),
            json_body: None,
        };
        let result: serde_json::Value = client
            .request(Method::GET, "/users/user_1/subscriptions", Some(&opts))
            .await
            .unwrap();
        assert_eq!(result["page"], 2);
    }

    #[tokio::test]
    async fn test_post_with_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/subscription/sub_1/cancel"))
            .and(body_json(serde_json::json!({"cancel_at_period_end": true})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sub_1",
                "object": "subscription_cancel",
                "success": true,
                "cancel_at_period_end": true
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let opts = RequestOptions {
            params: None,
            json_body: Some(serde_json::json!({"cancel_at_period_end": true})),
        };
        let result: serde_json::Value = client
            .request(Method::POST, "/subscription/sub_1/cancel", Some(&opts))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn test_custom_base_url_trailing_slash() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;

        let url_with_slash = format!("{}/", server.uri());
        let client = HttpClient::new("sk_test", &url_with_slash, DEFAULT_TIMEOUT);
        let result: serde_json::Value = client.request(Method::GET, "/test", None).await.unwrap();
        assert_eq!(result["ok"], true);
    }

    #[tokio::test]
    async fn test_timeout_returns_api_connection_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": true}))
                    .set_delay(Duration::from_secs(5)),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), Duration::from_millis(100));
        let err = client
            .request::<serde_json::Value>(Method::GET, "/slow", None)
            .await
            .unwrap_err();
        assert!(err.is_api_connection());
    }

    #[tokio::test]
    async fn test_connection_error() {
        let client = HttpClient::new("sk_test", "http://127.0.0.1:1", DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_api_connection());
    }

    #[tokio::test]
    async fn test_unrecognized_json_error_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"unknown_key": "some value"})),
            )
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert!(err.is_api());
        // Message should be the raw body since no recognized format
        assert!(err.message().contains("unknown_key"));
    }

    #[tokio::test]
    async fn test_v1_error_non_string_code() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": 123,
                    "message": "bad input"
                }
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        assert_eq!(err.message(), "bad input");
        assert_eq!(err.code(), None); // code is not a string
    }

    #[tokio::test]
    async fn test_v1_error_non_string_message() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "code": "err",
                    "message": 42
                }
            })))
            .mount(&server)
            .await;

        let client = HttpClient::new("sk_test", &server.uri(), DEFAULT_TIMEOUT);
        let err = client
            .request::<serde_json::Value>(Method::GET, "/test", None)
            .await
            .unwrap_err();
        // Falls back to raw body since message is not a string
        assert!(err.http_body().is_some());
    }
}
