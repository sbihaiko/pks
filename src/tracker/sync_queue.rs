use std::collections::VecDeque;
use std::time::Duration;

const MAX_BACKOFF_SECS: u64 = 16;

pub enum SyncOperation {
    Import { tracker_id: String, tracker_type: String },
    Export { file_path: String, tracker_type: String },
}

pub struct SyncQueue {
    queue: VecDeque<SyncOperation>,
}

impl SyncQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, op: SyncOperation) {
        self.queue.push_back(op);
    }

    pub fn enqueue_batch(&mut self, ops: Vec<SyncOperation>) {
        for op in ops {
            self.queue.push_back(op);
        }
    }

    pub fn next(&mut self) -> Option<SyncOperation> {
        self.queue.pop_front()
    }

    pub fn requeue_failed(&mut self, op: SyncOperation) {
        self.queue.push_back(op);
    }

    pub fn depth(&self) -> usize {
        self.queue.len()
    }

    pub fn backoff_duration(attempt: u32) -> Duration {
        let secs = (1u64 << attempt).min(MAX_BACKOFF_SECS);
        Duration::from_secs(secs)
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}
