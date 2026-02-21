/// The current version of the paylio-rust SDK.
pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
        assert!(!VERSION.is_empty());
    }
}
