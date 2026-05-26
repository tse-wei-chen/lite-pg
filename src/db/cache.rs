use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TimedCache<T: Clone> {
    data: Option<T>,
    fetched_at: Option<Instant>,
    pub ttl: Duration,
}

impl<T: Clone> TimedCache<T> {
    pub fn new(ttl_secs: u64) -> Self {
        TimedCache {
            data: None,
            fetched_at: None,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self) -> Option<&T> {
        match (&self.data, &self.fetched_at) {
            (Some(data), Some(time)) if time.elapsed() < self.ttl => Some(data),
            _ => None,
        }
    }

    pub fn set(&mut self, data: T) {
        self.data = Some(data);
        self.fetched_at = Some(Instant::now());
    }

}
