# Paylio Rust SDK

[![Crates.io](https://img.shields.io/crates/v/paylio.svg)](https://crates.io/crates/paylio)
[![CI](https://github.com/paylio-org/paylio-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/paylio-org/paylio-rust/actions/workflows/ci.yml)

The Paylio Rust SDK provides convenient access to the Paylio API from applications written in Rust.

## Documentation

See the [Paylio API docs](https://paylio.pro/docs).

## Requirements

- Rust 1.75+
- Tokio async runtime

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
paylio = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Usage

```rust
use paylio::Client;

#[tokio::main]
async fn main() -> Result<(), paylio::PaylioError> {
    let client = Client::new("sk_live_xxx")?;

    // Retrieve current subscription
    let sub = client.subscriptions().retrieve("user_123").await?;
    println!("Status: {}", sub.status);
    println!("Plan: {} ({})", sub.plan.name, sub.plan.slug);

    Ok(())
}
```

### List subscription history

```rust
use paylio::{Client, ListOptions};

let client = Client::new("sk_live_xxx")?;

// Default pagination
let list = client.subscriptions().list("user_123", None).await?;
println!("Total: {}, Page: {}/{}", list.total, list.page, list.total_pages);

for item in &list.items {
    println!("  {} — {} ({})", item.id, item.status, item.plan_slug);
}

// Custom pagination
let opts = ListOptions { page: Some(2), page_size: Some(5) };
let page2 = client.subscriptions().list("user_123", Some(&opts)).await?;
println!("Has more: {}", page2.has_more());
```

### Cancel a subscription

```rust
use paylio::{Client, CancelOptions};

let client = Client::new("sk_live_xxx")?;

// Cancel at end of billing period (safe default)
let result = client.subscriptions().cancel("sub_uuid", None).await?;
println!("Canceled: {}", result.success);

// Cancel immediately
let opts = CancelOptions { cancel_now: true };
let result = client.subscriptions().cancel("sub_uuid", Some(&opts)).await?;
```

### Configuration

```rust
use paylio::Client;
use std::time::Duration;

let client = Client::builder("sk_live_xxx")
    .base_url("https://custom-api.example.com/flying/v1")
    .timeout(Duration::from_secs(60))
    .build()?;
```

### Error handling

```rust
use paylio::{Client, PaylioError};

let client = Client::new("sk_live_xxx")?;

match client.subscriptions().retrieve("user_123").await {
    Ok(sub) => println!("Status: {}", sub.status),
    Err(e) => {
        println!("Error: {}", e);
        if e.is_authentication() {
            println!("Check your API key");
        } else if e.is_not_found() {
            println!("Subscription not found");
        } else if e.is_rate_limit() {
            println!("Rate limited, try again later");
        }
        if let Some(status) = e.http_status() {
            println!("HTTP status: {}", status);
        }
    }
}
```

## Error types

| HTTP Status | Variant | Description |
|-------------|---------|-------------|
| 401 | `PaylioError::Authentication` | Invalid or missing API key |
| 400 | `PaylioError::InvalidRequest` | Bad request parameters |
| 404 | `PaylioError::NotFound` | Resource not found |
| 429 | `PaylioError::RateLimit` | Rate limit exceeded |
| 5xx | `PaylioError::Api` | Server error |
| Network | `PaylioError::ApiConnection` | Connection or timeout error |

## Development

```bash
cargo test
cargo clippy
```

## License

MIT
