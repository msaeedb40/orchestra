//! Event collection, buffering, and subscription dispatch.

use orchestra_core::events::DomainEvent;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::debug;

/// Manages event logging, historical buffering, and live real-time subscription.
#[derive(Clone)]
pub struct EventCollector {
    history: Arc<Mutex<VecDeque<DomainEvent>>>,
    max_history: usize,
    sender: broadcast::Sender<DomainEvent>,
}

impl EventCollector {
    /// Creates a new EventCollector with a specified history buffer size and channel capacity.
    pub fn new(max_history: usize) -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            history: Arc::new(Mutex::new(VecDeque::with_capacity(max_history))),
            max_history,
            sender,
        }
    }

    /// Emits a new domain event into the history buffer and notifies active subscribers.
    pub fn publish(&self, event: DomainEvent) {
        debug!(event_id = %event.id, "Publishing domain event");

        // 1. Buffer in historical ring
        {
            let mut history = self.history.lock().unwrap();
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(event.clone());
        }

        // 2. Broadcast to live subscribers (ignore if no listeners)
        let _ = self.sender.send(event);
    }

    /// Subscribes to receive a live stream of domain events.
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }

    /// Returns a snapshot of buffered historical events.
    pub fn list_recent(&self, limit: usize) -> Vec<DomainEvent> {
        let history = self.history.lock().unwrap();
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Returns the number of events currently buffered.
    pub fn history_count(&self) -> usize {
        let history = self.history.lock().unwrap();
        history.len()
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self::new(500)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestra_core::events::{ClusterEvent, EventPayload};

    #[tokio::test]
    async fn test_event_collector_publish_and_subscribe() {
        let collector = EventCollector::new(10);
        let mut rx = collector.subscribe();

        let cluster_id = orchestra_common::identifiers::ClusterId::new("clus-1");
        let event = DomainEvent::new(
            Some(cluster_id.clone()),
            EventPayload::Cluster(ClusterEvent::Created {
                cluster_id,
                name: "test-clus".to_string(),
            }),
        );

        collector.publish(event.clone());

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);

        let recent = collector.list_recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, event.id);
    }
}
