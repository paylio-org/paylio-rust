//! # Paylio
//!
//! Official Rust client library for the [Paylio](https://paylio.pro) subscription API.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use paylio::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), paylio::PaylioError> {
//!     let client = Client::new("sk_live_xxx")?;
//!
//!     // Retrieve a subscription
//!     let sub = client.subscriptions().retrieve("user_123").await?;
//!     println!("Status: {}", sub.status);
//!
//!     // List subscription history
//!     let list = client.subscriptions().list("user_123", None).await?;
//!     println!("Total: {}", list.total);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub use error::PaylioError;
