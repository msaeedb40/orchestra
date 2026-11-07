//! eBPF link and attachment handles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

/// Represents an attached eBPF link (e.g. TC clsact qdisc filter or XDP netdev hook).
#[derive(Debug, Clone)]
pub struct AttachedLink {
    pub id: String,
    pub interface_name: String,
    pub hook_type: String,
}

/// LinkManager tracks active kernel hook attachments and cleans them up on shutdown.
#[derive(Debug, Clone, Default)]
pub struct LinkManager {
    links: Arc<Mutex<HashMap<String, AttachedLink>>>,
}

impl LinkManager {
    pub fn new() -> Self {
        Self {
            links: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Records an attachment link.
    pub fn attach(&self, link: AttachedLink) {
        info!(
            id = %link.id,
            iface = %link.interface_name,
            hook = %link.hook_type,
            "Attached eBPF program link"
        );
        let mut links = self.links.lock().unwrap();
        links.insert(link.id.clone(), link);
    }

    /// Detaches a link.
    pub fn detach(&self, id: &str) -> Option<AttachedLink> {
        let mut links = self.links.lock().unwrap();
        let detached = links.remove(id);
        if let Some(l) = &detached {
            info!(id = %l.id, iface = %l.interface_name, "Detached eBPF link");
        }
        detached
    }

    /// Detaches all links on teardown.
    pub fn detach_all(&self) {
        let mut links = self.links.lock().unwrap();
        for (_, l) in links.drain() {
            info!(id = %l.id, iface = %l.interface_name, "Detaching link during shutdown");
        }
    }

    /// Count of currently active attached links.
    pub fn active_count(&self) -> usize {
        let links = self.links.lock().unwrap();
        links.len()
    }
}
