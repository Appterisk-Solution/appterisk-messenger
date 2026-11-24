//! Local UI lock, not profile encryption. Passwords are hashed with Argon2id and OS-random salts.
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use std::time::{Duration, Instant};
pub struct AppLock {
    pub hash: Option<String>,
    pub locked: bool,
    pub last_activity: Instant,
    failures: u32,
    retry_after: Instant,
}
impl AppLock {
    pub fn new(hash: Option<String>) -> Self {
        let locked = hash.is_some();
        Self {
            hash,
            locked,
            last_activity: Instant::now(),
            failures: 0,
            retry_after: Instant::now(),
        }
    }
    pub fn allowed(&self) -> bool {
        Instant::now() >= self.retry_after
    }
    pub fn failed(&mut self) {
        self.failures = self.failures.saturating_add(1);
        self.retry_after =
            Instant::now() + Duration::from_secs((1u64 << self.failures.min(5)).min(30));
    }
    pub fn unlocked(&mut self) {
        self.locked = false;
        self.failures = 0;
        self.retry_after = Instant::now();
        self.last_activity = Instant::now();
    }
    pub fn should_lock(&self, seconds: u32) -> bool {
        self.hash.is_some()
            && !self.locked
            && seconds > 0
            && self.last_activity.elapsed() >= Duration::from_secs(u64::from(seconds))
    }
}
pub fn hash(password: &str) -> Result<String, String> {
    if password.chars().count() < 8 || password.len() > 1024 {
        return Err("Use a password of at least 8 characters (maximum 1024 bytes).".into());
    }
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h: PasswordHash| h.to_string())
        .map_err(|_| "Password hashing failed.".into())
}
pub fn verify(password: &str, hash: &str) -> bool {
    if password.len() > 1024 {
        return false;
    }
    PasswordHash::new(hash).ok().is_some_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn salted_hashes_and_lock_state() {
        let a = hash("a strong test password").unwrap();
        let b = hash("a strong test password").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("$argon2id$"));
        assert!(verify("a strong test password", &a));
        assert!(!verify("incorrect", &a));
        assert!(!a.contains("a strong test password"));
        let mut lock = AppLock::new(Some(a));
        assert!(lock.locked);
        lock.unlocked();
        lock.last_activity = Instant::now() - Duration::from_secs(61);
        assert!(lock.should_lock(60));
        assert!(!lock.should_lock(0));
        lock.failed();
        assert!(!lock.allowed());
        assert!(hash("short").is_err());
    }
}
