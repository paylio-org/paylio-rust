use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Method;

use crate::error::PaylioError;
use crate::http_client::{HttpClient, RequestOptions};
use crate::resources::{PaginatedList, Subscription, SubscriptionCancel, SubscriptionHistoryItem};

/// Options for listing subscription history.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Page number (defaults to 1).
    pub page: Option<i64>,
    /// Number of items per page (defaults to 20).
    pub page_size: Option<i64>,
}

/// Options for canceling a subscription.
#[derive(Debug, Clone, Default)]
pub struct CancelOptions {
    /// If true, cancel immediately instead of at the end of the billing period.
    pub cancel_now: bool,
}

/// Service for managing subscriptions.
pub struct SubscriptionService {
    client: Arc<HttpClient>,
}

impl SubscriptionService {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Retrieve the current subscription for a user.
    pub async fn retrieve(&self, user_id: &str) -> Result<Subscription, PaylioError> {
        let trimmed = user_id.trim();
        if trimmed.is_empty() {
            return Err(PaylioError::InvalidRequest {
                message: "user_id must not be empty".to_string(),
                http_status: None,
                http_body: None,
                code: None,
            });
        }

        self.client
            .request(Method::GET, &format!("/subscription/{}", trimmed), None)
            .await
    }

    /// List subscription history for a user with optional pagination.
    pub async fn list(
        &self,
        user_id: &str,
        opts: Option<&ListOptions>,
    ) -> Result<PaginatedList<SubscriptionHistoryItem>, PaylioError> {
        let trimmed = user_id.trim();
        if trimmed.is_empty() {
            return Err(PaylioError::InvalidRequest {
                message: "user_id must not be empty".to_string(),
                http_status: None,
                http_body: None,
                code: None,
            });
        }

        let page = opts.and_then(|o| o.page).unwrap_or(1);
        let page_size = opts.and_then(|o| o.page_size).unwrap_or(20);

        let mut params = HashMap::new();
        params.insert("page".to_string(), page.to_string());
        params.insert("page_size".to_string(), page_size.to_string());

        let request_opts = RequestOptions {
            params: Some(params),
            json_body: None,
        };

        self.client
            .request(
                Method::GET,
                &format!("/users/{}/subscriptions", trimmed),
                Some(&request_opts),
            )
            .await
    }

    /// Cancel a subscription.
    pub async fn cancel(
        &self,
        subscription_id: &str,
        opts: Option<&CancelOptions>,
    ) -> Result<SubscriptionCancel, PaylioError> {
        let trimmed = subscription_id.trim();
        if trimmed.is_empty() {
            return Err(PaylioError::InvalidRequest {
                message: "subscription_id must not be empty".to_string(),
                http_status: None,
                http_body: None,
                code: None,
            });
        }

        let cancel_at_period_end = opts.is_none_or(|o| !o.cancel_now);

        let request_opts = RequestOptions {
            params: None,
            json_body: Some(serde_json::json!({ "cancel_at_period_end": cancel_at_period_end })),
        };

        self.client
            .request(
                Method::POST,
                &format!("/subscription/{}/cancel", trimmed),
                Some(&request_opts),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http_client::DEFAULT_TIMEOUT;

    fn make_service(base_url: &str) -> SubscriptionService {
        let http = HttpClient::new("sk_test_key", base_url, DEFAULT_TIMEOUT);
        SubscriptionService::new(Arc::new(http))
    }

    #[tokio::test]
    async fn test_retrieve_returns_subscription() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/subscription/user_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sub_1",
                "object": "subscription",
                "status": "active",
                "user_id": "user_123",
                "plan": {"slug": "pro", "name": "Pro", "interval": "month", "amount": 9.99, "currency": "usd"},
                "subscription_period": {"start": "2026-01-01", "end": "2026-02-01"},
                "cancel_at_period_end": false,
                "canceled_at": null,
                "provider": "stripe",
                "created_at": "2025-12-01"
            })))
            .mount(&server)
            .await;

        let svc = make_service(&server.uri());
        let sub = svc.retrieve("user_123").await.unwrap();
        assert_eq!(sub.id, "sub_1");
        assert_eq!(sub.status, "active");
        assert_eq!(sub.user_id, "user_123");
        assert_eq!(sub.plan.slug, "pro");
    }

    #[tokio::test]
    async fn test_retrieve_empty_user_id() {
        let server = MockServer::start().await;
        let svc = make_service(&server.uri());
        let err = svc.retrieve("").await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "user_id must not be empty");
    }

    #[tokio::test]
    async fn test_retrieve_whitespace_user_id() {
        let server = MockServer::start().await;
        let svc = make_service(&server.uri());
        let err = svc.retrieve("   ").await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "user_id must not be empty");
    }

    #[tokio::test]
    async fn test_retrieve_propagates_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/subscription/user_bad"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "Not found"})),
            )
            .mount(&server)
            .await;

        let svc = make_service(&server.uri());
        let err = svc.retrieve("user_bad").await.unwrap_err();
        assert!(err.is_not_found());
    }

    #[tokio::test]
    async fn test_list_returns_paginated_results() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/user_1/subscriptions"))
            .and(query_param("page", "2"))
            .and(query_param("page_size", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{
                    "id": "sub_hist_1",
                    "user_id": "user_1",
                    "plan_slug": "pro",
                    "plan_name": "Pro",
                    "plan_amount": 9.99,
                    "plan_currency": "usd",
                    "plan_interval": "month",
                    "status": "active",
                    "current_period_start": "2026-01-01",
                    "current_period_end": "2026-02-01",
                    "created_at": "2025-12-01"
                }],
                "total": 10,
                "page": 2,
                "page_size": 5,
                "total_pages": 2
            })))
            .mount(&server)
            .await;

        let svc = make_service(&server.uri());
        let opts = ListOptions {
            page: Some(2),
            page_size: Some(5),
        };
        let list = svc.list("user_1", Some(&opts)).await.unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.total, 10);
        assert_eq!(list.page, 2);
        assert_eq!(list.page_size, 5);
    }

    #[tokio::test]
    async fn test_list_default_pagination() {
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

        let svc = make_service(&server.uri());
        let list = svc.list("user_1", None).await.unwrap();
        assert_eq!(list.page, 1);
        assert_eq!(list.page_size, 20);
    }

    #[tokio::test]
    async fn test_list_empty_user_id() {
        let server = MockServer::start().await;
        let svc = make_service(&server.uri());
        let err = svc.list("", None).await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "user_id must not be empty");
    }

    #[tokio::test]
    async fn test_cancel_default_at_period_end() {
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

        let svc = make_service(&server.uri());
        let result = svc.cancel("sub_1", None).await.unwrap();
        assert_eq!(result.id, "sub_1");
        assert!(result.success);
        assert!(result.cancel_at_period_end);
    }

    #[tokio::test]
    async fn test_cancel_immediate() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/subscription/sub_2/cancel"))
            .and(body_json(
                serde_json::json!({"cancel_at_period_end": false}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sub_2",
                "object": "subscription_cancel",
                "success": true,
                "cancel_at_period_end": false
            })))
            .mount(&server)
            .await;

        let svc = make_service(&server.uri());
        let opts = CancelOptions { cancel_now: true };
        let result = svc.cancel("sub_2", Some(&opts)).await.unwrap();
        assert_eq!(result.id, "sub_2");
        assert!(result.success);
        assert!(!result.cancel_at_period_end);
    }

    #[tokio::test]
    async fn test_cancel_empty_subscription_id() {
        let server = MockServer::start().await;
        let svc = make_service(&server.uri());
        let err = svc.cancel("", None).await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "subscription_id must not be empty");
    }

    #[tokio::test]
    async fn test_cancel_whitespace_subscription_id() {
        let server = MockServer::start().await;
        let svc = make_service(&server.uri());
        let err = svc.cancel("  ", None).await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "subscription_id must not be empty");
    }

    #[tokio::test]
    async fn test_cancel_propagates_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/subscription/sub_bad/cancel"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"error": "Invalid subscription"})),
            )
            .mount(&server)
            .await;

        let svc = make_service(&server.uri());
        let err = svc.cancel("sub_bad", None).await.unwrap_err();
        assert!(err.is_invalid_request());
        assert_eq!(err.message(), "Invalid subscription");
    }
}
