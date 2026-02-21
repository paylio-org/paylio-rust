use std::sync::Arc;
use std::time::Duration;

use crate::error::PaylioError;
use crate::http_client::{HttpClient, DEFAULT_BASE_URL, DEFAULT_TIMEOUT};
use crate::subscription::SubscriptionService;

/// A client for the Paylio API.
///
/// # Examples
///
/// ```rust,ignore
/// let client = paylio::Client::new("sk_live_xxx")?;
/// let sub = client.subscriptions().retrieve("user_123").await?;
/// ```
pub struct Client {
    subscriptions: SubscriptionService,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish()
    }
}

impl Client {
    /// Create a new client with the given API key and default settings.
    pub fn new(api_key: &str) -> Result<Self, PaylioError> {
        ClientBuilder::new(api_key).build()
    }

    /// Create a builder for more advanced configuration.
    pub fn builder(api_key: &str) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Access the subscription service.
    pub fn subscriptions(&self) -> &SubscriptionService {
        &self.subscriptions
    }
}

/// Builder for configuring a [`Client`].
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
    timeout: Duration,
}

impl ClientBuilder {
    fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Set a custom base URL (e.g., for testing).
    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Set a custom request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client, PaylioError> {
        let trimmed = self.api_key.trim();
        if trimmed.is_empty() {
            return Err(PaylioError::Authentication {
                message: "API key must not be empty".to_string(),
                http_status: None,
                http_body: None,
                code: None,
            });
        }

        let http = Arc::new(HttpClient::new(trimmed, &self.base_url, self.timeout));
        let subscriptions = SubscriptionService::new(Arc::clone(&http));

        Ok(Client { subscriptions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_valid_key() {
        let client = Client::new("sk_test_abc");
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_with_empty_key() {
        let err = Client::new("").unwrap_err();
        assert!(err.is_authentication());
        assert_eq!(err.message(), "API key must not be empty");
    }

    #[test]
    fn test_new_with_whitespace_key() {
        let err = Client::new("   ").unwrap_err();
        assert!(err.is_authentication());
        assert_eq!(err.message(), "API key must not be empty");
    }

    #[test]
    fn test_builder_with_custom_base_url() {
        let client = Client::builder("sk_test_abc")
            .base_url("https://custom.api.com/v1")
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_custom_timeout() {
        let client = Client::builder("sk_test_abc")
            .timeout(Duration::from_secs(60))
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_all_options() {
        let client = Client::builder("sk_test_abc")
            .base_url("https://custom.api.com/v1")
            .timeout(Duration::from_secs(60))
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_empty_key() {
        let err = Client::builder("").build().unwrap_err();
        assert!(err.is_authentication());
    }

    #[test]
    fn test_subscriptions_accessor() {
        let client = Client::new("sk_test_abc").unwrap();
        // Just verify we can access subscriptions (type check)
        let _svc = client.subscriptions();
    }

    #[tokio::test]
    async fn test_client_retrieve_via_wiremock() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/subscription/user_1"))
            .and(header("X-API-Key", "sk_test_key"))
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

        let client = Client::builder("sk_test_key")
            .base_url(&server.uri())
            .build()
            .unwrap();
        let sub = client.subscriptions().retrieve("user_1").await.unwrap();
        assert_eq!(sub.id, "sub_1");
        assert_eq!(sub.status, "active");
    }

    #[tokio::test]
    async fn test_client_list_via_wiremock() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/user_1/subscriptions"))
            .and(query_param("page", "1"))
            .and(query_param("page_size", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "total": 0,
                "page": 1,
                "page_size": 20,
                "total_pages": 0
            })))
            .mount(&server)
            .await;

        let client = Client::builder("sk_test_key")
            .base_url(&server.uri())
            .build()
            .unwrap();
        let list = client.subscriptions().list("user_1", None).await.unwrap();
        assert_eq!(list.total, 0);
        assert_eq!(list.page, 1);
    }

    #[tokio::test]
    async fn test_client_cancel_via_wiremock() {
        use wiremock::matchers::{body_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

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

        let client = Client::builder("sk_test_key")
            .base_url(&server.uri())
            .build()
            .unwrap();
        let result = client.subscriptions().cancel("sub_1", None).await.unwrap();
        assert!(result.success);
    }
}
