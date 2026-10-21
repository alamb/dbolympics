//! Resources with a daily capacity (warehouses, carrier networks, support desks).
//!
//! Capacity is sized to the resource's recent normal volume (trailing 8 weeks) times some
//! headroom, not to peaks. Each day the previous day's actual volume joins the backlog; what
//! exceeds capacity carries over, and the backlog in days of work is the extra delay. Peak
//! seasons, outages (capacity factor < 1) and understaffing therefore cause delays on their
//! own, and the delays feed into experience, tickets and loyalty.

use std::collections::VecDeque;

const WINDOW: usize = 56;

#[derive(Default)]
pub struct Queue {
    hist: VecDeque<f64>,
    backlog: f64,
    /// current backlog, in days of work at today's capacity
    pub delay_days: f64,
}

impl Queue {
    /// Feed yesterday's volume. `headroom` sizes capacity relative to the trailing mean,
    /// `factor` scales it (e.g. 0.4 during a strike, 1.3 with seasonal staff).
    pub fn step(&mut self, volume: f64, headroom: f64, factor: f64) -> f64 {
        let mean = if self.hist.is_empty() { volume } else { self.hist.iter().sum::<f64>() / self.hist.len() as f64 };
        let capacity = (mean * headroom).max(1.0) * factor;
        let load = self.backlog + volume;
        self.backlog = (load - capacity).max(0.0);
        self.hist.push_back(volume);
        if self.hist.len() > WINDOW {
            self.hist.pop_front();
        }
        self.delay_days = self.backlog / capacity;
        self.delay_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_builds_backlog_then_drains() {
        let mut q = Queue::default();
        for _ in 0..60 {
            q.step(100.0, 1.25, 1.0);
        }
        assert_eq!(q.delay_days, 0.0);
        q.step(400.0, 1.25, 1.0); // Black Friday
        assert!(q.delay_days > 1.0);
        let mut d = q.delay_days;
        for _ in 0..10 {
            q.step(100.0, 1.25, 1.0);
            assert!(q.delay_days <= d);
            d = q.delay_days;
        }
        assert!(d < 0.5);
    }
}
