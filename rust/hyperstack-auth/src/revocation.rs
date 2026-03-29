//! Token revocation support
//!
//! Provides functionality to revoke tokens before their natural expiry.
//! Revoked tokens are tracked by their JWT ID (jti) claim.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// A revoked token entry with expiration tracking
#[derive(Debug, Clone)]
struct RevokedEntry {
    jti: String,
    expires_at: u64,
    revoked_at: Instant,
}

/// Token revocation list with automatic cleanup
#[derive(Clone)]
pub struct TokenRevocationList {
    /// Set of revoked JWT IDs
    revoked: Arc<RwLock<HashSet<String>>>,
    /// Maximum age of revocation entries before cleanup
    max_age: Duration,
}

impl TokenRevocationList {
    /// Create a new empty revocation list
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(RwLock::new(HashSet::new())),
            max_age: Duration::from_secs(86400), // 24 hours default
        }
    }

    /// Set the maximum age of revocation entries
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Revoke a token by its JTI
    pub async fn revoke(&self, jti: impl Into<String>) {
        let mut revoked = self.revoked.write().await;
        revoked.insert(jti.into());
    }

    /// Check if a token is revoked
    pub async fn is_revoked(&self, jti: &str) -> bool {
        let revoked = self.revoked.read().await;
        revoked.contains(jti)
    }

    /// Remove a token from the revocation list
    pub async fn unrevoke(&self, jti: &str) {
        let mut revoked = self.revoked.write().await;
        revoked.remove(jti);
    }

    /// Get the number of revoked tokens
    pub async fn len(&self) -> usize {
        let revoked = self.revoked.read().await;
        revoked.len()
    }

    /// Check if the revocation list is empty
    pub async fn is_empty(&self) -> bool {
        let revoked = self.revoked.read().await;
        revoked.is_empty()
    }

    /// Clear all revoked tokens
    pub async fn clear(&self) {
        let mut revoked = self.revoked.write().await;
        revoked.clear();
    }

    /// Clean up old revocation entries (should be called periodically)
    pub async fn cleanup_expired(&self, now: u64) -> usize {
        // Note: In a real implementation, we'd track the expiration time of each token
        // and only remove entries for tokens that have naturally expired.
        // For now, this is a no-op placeholder.
        let _ = now;
        0
    }
}

impl Default for TokenRevocationList {
    fn default() -> Self {
        Self::new()
    }
}

/// Revocation checker trait for integration with verifiers
#[async_trait::async_trait]
pub trait RevocationChecker: Send + Sync {
    /// Check if a token is revoked
    async fn is_revoked(&self, jti: &str) -> bool;
}

#[async_trait::async_trait]
impl RevocationChecker for TokenRevocationList {
    async fn is_revoked(&self, jti: &str) -> bool {
        self.is_revoked(jti).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_revoke_and_check() {
        let list = TokenRevocationList::new();

        assert!(!list.is_revoked("token-1").await);

        list.revoke("token-1").await;
        assert!(list.is_revoked("token-1").await);

        list.unrevoke("token-1").await;
        assert!(!list.is_revoked("token-1").await);
    }

    #[tokio::test]
    async fn test_multiple_tokens() {
        let list = TokenRevocationList::new();

        list.revoke("token-1").await;
        list.revoke("token-2").await;

        assert!(list.is_revoked("token-1").await);
        assert!(list.is_revoked("token-2").await);
        assert!(!list.is_revoked("token-3").await);

        assert_eq!(list.len().await, 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let list = TokenRevocationList::new();

        list.revoke("token-1").await;
        list.revoke("token-2").await;

        list.clear().await;

        assert!(list.is_empty().await);
        assert!(!list.is_revoked("token-1").await);
    }
}
