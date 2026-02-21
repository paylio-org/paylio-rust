use serde::{Deserialize, Serialize};

/// Subscription plan details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub slug: String,
    pub name: String,
    pub interval: String,
    pub amount: f64,
    pub currency: String,
}

/// Subscription time period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Period {
    pub start: String,
    pub end: String,
}

/// A subscription resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub object: String,
    pub status: String,
    pub user_id: String,
    pub plan: Plan,
    pub subscription_period: Period,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<String>,
    pub provider: String,
    pub created_at: String,
}

/// Result of canceling a subscription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionCancel {
    pub id: String,
    pub object: String,
    pub success: bool,
    pub cancel_at_period_end: bool,
}

/// A subscription history entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionHistoryItem {
    pub id: String,
    pub user_id: String,
    pub plan_slug: String,
    pub plan_name: Option<String>,
    pub plan_amount: Option<f64>,
    pub plan_currency: String,
    pub plan_interval: String,
    pub status: String,
    pub current_period_start: String,
    pub current_period_end: String,
    pub created_at: String,
}

/// A paginated list of items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PaginatedList<T> {
    /// Returns true if there are more pages after the current one.
    pub fn has_more(&self) -> bool {
        self.page > 0 && self.page < self.total_pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_deserialize() {
        let json = r#"{
            "slug": "pro",
            "name": "Pro Plan",
            "interval": "month",
            "amount": 9.99,
            "currency": "usd"
        }"#;
        let plan: Plan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.slug, "pro");
        assert_eq!(plan.name, "Pro Plan");
        assert_eq!(plan.interval, "month");
        assert!((plan.amount - 9.99).abs() < f64::EPSILON);
        assert_eq!(plan.currency, "usd");
    }

    #[test]
    fn test_subscription_deserialize() {
        let json = r#"{
            "id": "sub_123",
            "object": "subscription",
            "status": "active",
            "user_id": "user_456",
            "plan": {
                "slug": "pro",
                "name": "Pro",
                "interval": "month",
                "amount": 9.99,
                "currency": "usd"
            },
            "subscription_period": {
                "start": "2026-01-01T00:00:00Z",
                "end": "2026-02-01T00:00:00Z"
            },
            "cancel_at_period_end": false,
            "canceled_at": null,
            "provider": "stripe",
            "created_at": "2025-12-01T00:00:00Z"
        }"#;
        let sub: Subscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.id, "sub_123");
        assert_eq!(sub.object, "subscription");
        assert_eq!(sub.status, "active");
        assert_eq!(sub.user_id, "user_456");
        assert_eq!(sub.plan.slug, "pro");
        assert_eq!(sub.subscription_period.start, "2026-01-01T00:00:00Z");
        assert!(!sub.cancel_at_period_end);
        assert!(sub.canceled_at.is_none());
        assert_eq!(sub.provider, "stripe");
    }

    #[test]
    fn test_subscription_canceled_at_present() {
        let json = r#"{
            "id": "sub_789",
            "object": "subscription",
            "status": "canceled",
            "user_id": "user_456",
            "plan": {
                "slug": "pro",
                "name": "Pro",
                "interval": "month",
                "amount": 9.99,
                "currency": "usd"
            },
            "subscription_period": {
                "start": "2026-01-01T00:00:00Z",
                "end": "2026-02-01T00:00:00Z"
            },
            "cancel_at_period_end": true,
            "canceled_at": "2026-01-15T12:00:00Z",
            "provider": "stripe",
            "created_at": "2025-12-01T00:00:00Z"
        }"#;
        let sub: Subscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.canceled_at, Some("2026-01-15T12:00:00Z".to_string()));
        assert!(sub.cancel_at_period_end);
    }

    #[test]
    fn test_subscription_serde_round_trip() {
        let sub = Subscription {
            id: "sub_1".into(),
            object: "subscription".into(),
            status: "active".into(),
            user_id: "user_1".into(),
            plan: Plan {
                slug: "basic".into(),
                name: "Basic".into(),
                interval: "month".into(),
                amount: 5.0,
                currency: "usd".into(),
            },
            subscription_period: Period {
                start: "2026-01-01".into(),
                end: "2026-02-01".into(),
            },
            cancel_at_period_end: false,
            canceled_at: None,
            provider: "stripe".into(),
            created_at: "2025-12-01".into(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        let deserialized: Subscription = serde_json::from_str(&json).unwrap();
        assert_eq!(sub, deserialized);
    }

    #[test]
    fn test_subscription_cancel_deserialize() {
        let json = r#"{
            "id": "sub_123",
            "object": "subscription_cancel",
            "success": true,
            "cancel_at_period_end": true
        }"#;
        let cancel: SubscriptionCancel = serde_json::from_str(json).unwrap();
        assert_eq!(cancel.id, "sub_123");
        assert_eq!(cancel.object, "subscription_cancel");
        assert!(cancel.success);
        assert!(cancel.cancel_at_period_end);
    }

    #[test]
    fn test_subscription_history_item_deserialize() {
        let json = r#"{
            "id": "sub_hist_1",
            "user_id": "user_1",
            "plan_slug": "pro",
            "plan_name": "Pro Plan",
            "plan_amount": 19.99,
            "plan_currency": "usd",
            "plan_interval": "month",
            "status": "active",
            "current_period_start": "2026-01-01T00:00:00Z",
            "current_period_end": "2026-02-01T00:00:00Z",
            "created_at": "2025-12-01T00:00:00Z"
        }"#;
        let item: SubscriptionHistoryItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "sub_hist_1");
        assert_eq!(item.plan_slug, "pro");
        assert_eq!(item.plan_name, Some("Pro Plan".to_string()));
        assert!((item.plan_amount.unwrap() - 19.99).abs() < f64::EPSILON);
    }

    #[test]
    fn test_paginated_list_deserialize() {
        let json = r#"{
            "items": [
                {
                    "id": "sub_1",
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
                }
            ],
            "total": 10,
            "page": 1,
            "page_size": 1,
            "total_pages": 10
        }"#;
        let list: PaginatedList<SubscriptionHistoryItem> = serde_json::from_str(json).unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.total, 10);
        assert_eq!(list.page, 1);
        assert_eq!(list.page_size, 1);
        assert_eq!(list.total_pages, 10);
    }

    #[test]
    fn test_has_more_true() {
        let list: PaginatedList<String> = PaginatedList {
            items: vec![],
            total: 30,
            page: 1,
            page_size: 10,
            total_pages: 3,
        };
        assert!(list.has_more());
    }

    #[test]
    fn test_has_more_false_last_page() {
        let list: PaginatedList<String> = PaginatedList {
            items: vec![],
            total: 30,
            page: 3,
            page_size: 10,
            total_pages: 3,
        };
        assert!(!list.has_more());
    }

    #[test]
    fn test_has_more_false_zero_page() {
        let list: PaginatedList<String> = PaginatedList {
            items: vec![],
            total: 0,
            page: 0,
            page_size: 10,
            total_pages: 0,
        };
        assert!(!list.has_more());
    }

    #[test]
    fn test_has_more_false_single_page() {
        let list: PaginatedList<String> = PaginatedList {
            items: vec![],
            total: 5,
            page: 1,
            page_size: 10,
            total_pages: 1,
        };
        assert!(!list.has_more());
    }

    #[test]
    fn test_subscription_history_item_without_optional_fields() {
        let json = r#"{
            "id": "sub_hist_2",
            "user_id": "user_1",
            "plan_slug": "pro",
            "plan_currency": "usd",
            "plan_interval": "month",
            "status": "active",
            "current_period_start": "2026-01-01T00:00:00Z",
            "current_period_end": "2026-02-01T00:00:00Z",
            "created_at": "2025-12-01T00:00:00Z"
        }"#;
        let item: SubscriptionHistoryItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "sub_hist_2");
        assert_eq!(item.plan_slug, "pro");
        assert!(item.plan_name.is_none());
        assert!(item.plan_amount.is_none());
    }

    #[test]
    fn test_has_more_page_2_of_5() {
        let list: PaginatedList<String> = PaginatedList {
            items: vec![],
            total: 50,
            page: 2,
            page_size: 10,
            total_pages: 5,
        };
        assert!(list.has_more());
    }
}
