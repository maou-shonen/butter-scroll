use crate::threshold::AppKey;

/// Resolves a process ID to an application identity for threshold caching.
pub trait ProcessResolver: Send + Sync {
    fn resolve_pid(&self, pid: u32) -> Option<AppKey>;
}

#[cfg(test)]
pub struct MockProcessResolver {
    pub result: Option<AppKey>,
}

#[cfg(test)]
impl ProcessResolver for MockProcessResolver {
    fn resolve_pid(&self, _pid: u32) -> Option<AppKey> {
        self.result.clone()
    }
}
