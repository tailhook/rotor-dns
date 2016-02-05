use std::cmp::{max, Ordering};
use time::SteadyTime;


use {TimeEntry};


impl TimeEntry {
    pub fn millis(&self) -> u64 {
        return max(0, (self.0 - SteadyTime::now()).num_milliseconds()) as u64
    }
}

// This implements reverse Ord on time, so BinaryHeap will return minimal
// time not the maximum one
impl PartialOrd for TimeEntry {
    fn partial_cmp(&self, other: &TimeEntry) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

// This implements reverse Ord on time, so BinaryHeap will return minimal
// time not the maximum one
impl Ord for TimeEntry {
    fn cmp(&self, other: &TimeEntry) -> Ordering {
        other.0.cmp(&self.0)
    }
}

impl PartialEq for TimeEntry {
    fn eq(&self, other: &TimeEntry) -> bool {
        other.0.eq(&self.0)
    }
}

impl Eq for TimeEntry {}
