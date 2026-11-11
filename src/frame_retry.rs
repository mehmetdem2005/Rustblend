//! Bounded delayed retries; a failed surface must not create a hot redraw loop.
use std::time::{Duration, Instant};
#[derive(Default)]
pub struct FrameRetry { remaining: u8, deadline: Option<Instant> }
impl FrameRetry {
    pub fn reset(&mut self) { self.remaining=3; self.deadline=None; }
    pub fn clear(&mut self) { self.deadline=None; }
    pub fn failed(&mut self, now:Instant) {
        if self.remaining>0 { self.remaining-=1; self.deadline=Some(now+Duration::from_millis(100)); }
        else { self.deadline=None; }
    }
    pub fn deadline(&self)->Option<Instant> { self.deadline }
    pub fn take_due(&mut self, now:Instant)->bool {
        if self.deadline.is_some_and(|deadline| now>=deadline) { self.deadline=None; true } else {false}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_retry_only_three_times_and_never_immediately() {
        let mut policy=FrameRetry::default(); policy.reset(); let mut now=Instant::now();
        for _ in 0..3 { policy.failed(now); assert!(!policy.take_due(now)); now+=Duration::from_millis(100); assert!(policy.take_due(now)); assert!(!policy.take_due(now)); }
        policy.failed(now); assert!(policy.deadline().is_none());
        policy.reset(); policy.failed(now); assert!(policy.deadline().is_some());
    }
    #[test]
    fn success_or_suspend_cancels_scheduled_retry() {
        let mut policy=FrameRetry::default(); let now=Instant::now();
        policy.reset(); policy.failed(now); policy.clear();
        assert!(!policy.take_due(now+Duration::from_secs(1)));
    }
}
