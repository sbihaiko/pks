#[path = "../src/tracker/sync_queue.rs"]
mod sync_queue;

use sync_queue::{SyncOperation, SyncQueue};
use std::time::Duration;

fn import_op(id: &str) -> SyncOperation {
    SyncOperation::Import {
        tracker_id: id.to_string(),
        tracker_type: "jira".to_string(),
    }
}

fn export_op(path: &str) -> SyncOperation {
    SyncOperation::Export {
        file_path: path.to_string(),
        tracker_type: "csv".to_string(),
    }
}

#[test]
fn enqueue_dequeue_preserves_fifo_order() {
    let mut q = SyncQueue::new();
    q.enqueue(import_op("first"));
    q.enqueue(import_op("second"));
    q.enqueue(import_op("third"));

    let first = q.next().unwrap();
    let second = q.next().unwrap();
    let third = q.next().unwrap();

    let id1 = match first { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };
    let id2 = match second { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };
    let id3 = match third { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };

    assert_eq!(id1, "first");
    assert_eq!(id2, "second");
    assert_eq!(id3, "third");
}

#[test]
fn next_on_empty_queue_returns_none() {
    let mut q = SyncQueue::new();
    assert!(q.next().is_none());
}

#[test]
fn enqueue_batch_adds_all_in_order() {
    let mut q = SyncQueue::new();
    let ops = vec![import_op("a"), import_op("b"), export_op("c.csv")];
    q.enqueue_batch(ops);

    assert_eq!(q.depth(), 3);

    let first = q.next().unwrap();
    let tracker_id = match first { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };
    assert_eq!(tracker_id, "a");
}

#[test]
fn requeue_failed_places_op_at_end() {
    let mut q = SyncQueue::new();
    q.enqueue(import_op("first"));
    q.enqueue(import_op("second"));

    let failed = q.next().unwrap();
    q.requeue_failed(failed);

    let next = q.next().unwrap();
    let tracker_id = match next { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };
    assert_eq!(tracker_id, "second");

    let requeued = q.next().unwrap();
    let requeued_id = match requeued { SyncOperation::Import { tracker_id, .. } => tracker_id, _ => unreachable!() };
    assert_eq!(requeued_id, "first");
}

#[test]
fn depth_reflects_queue_size() {
    let mut q = SyncQueue::new();
    assert_eq!(q.depth(), 0);

    q.enqueue(import_op("x"));
    assert_eq!(q.depth(), 1);

    q.enqueue(import_op("y"));
    assert_eq!(q.depth(), 2);

    q.next();
    assert_eq!(q.depth(), 1);
}

#[test]
fn backoff_duration_grows_exponentially() {
    assert_eq!(SyncQueue::backoff_duration(0), Duration::from_secs(1));
    assert_eq!(SyncQueue::backoff_duration(1), Duration::from_secs(2));
    assert_eq!(SyncQueue::backoff_duration(2), Duration::from_secs(4));
    assert_eq!(SyncQueue::backoff_duration(3), Duration::from_secs(8));
    assert_eq!(SyncQueue::backoff_duration(4), Duration::from_secs(16));
}

#[test]
fn backoff_duration_caps_at_max() {
    assert_eq!(SyncQueue::backoff_duration(5), Duration::from_secs(16));
    assert_eq!(SyncQueue::backoff_duration(10), Duration::from_secs(16));
    assert_eq!(SyncQueue::backoff_duration(30), Duration::from_secs(16));
}

#[test]
fn enqueue_batch_empty_vec_does_not_change_depth() {
    let mut q = SyncQueue::new();
    q.enqueue_batch(vec![]);
    assert_eq!(q.depth(), 0);
}
