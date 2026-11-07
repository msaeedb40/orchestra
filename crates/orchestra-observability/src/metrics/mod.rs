//! Prometheus metrics exposition, counters, gauges, and formatters.

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Metrics registry tracking counters, gauges, and histograms for Prometheus scraping.
#[derive(Debug, Clone, Default)]
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Arc<AtomicU64>>>>,
    gauges: Arc<RwLock<HashMap<String, Arc<AtomicI64>>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize core default metrics
        registry.register_counter("orchestra_packets_processed_total");
        registry.register_counter("orchestra_packets_dropped_total");
        registry.register_counter("orchestra_reconcile_runs_total");
        registry.register_gauge("orchestra_clusters_active");
        registry.register_gauge("orchestra_nodes_active");
        registry.register_gauge("orchestra_ebpf_maps_count");

        registry
    }

    /// Registers a new counter metric.
    pub fn register_counter(&self, name: &str) {
        let mut counters = self.counters.write().unwrap();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)));
    }

    /// Increments a counter by a given amount.
    pub fn inc_counter(&self, name: &str, amount: u64) {
        let counters = self.counters.read().unwrap();
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(amount, Ordering::Relaxed);
        } else {
            drop(counters);
            self.register_counter(name);
            self.inc_counter(name, amount);
        }
    }

    /// Registers a new gauge metric.
    pub fn register_gauge(&self, name: &str) {
        let mut gauges = self.gauges.write().unwrap();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicI64::new(0)));
    }

    /// Sets the value of a gauge.
    pub fn set_gauge(&self, name: &str, value: i64) {
        let gauges = self.gauges.read().unwrap();
        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            self.register_gauge(name);
            self.set_gauge(name, value);
        }
    }

    /// Reads a counter value.
    pub fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read().unwrap();
        counters
            .get(name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Reads a gauge value.
    pub fn get_gauge(&self, name: &str) -> i64 {
        let gauges = self.gauges.read().unwrap();
        gauges
            .get(name)
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Formats all registered metrics into Prometheus text format.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::new();

        let counters = self.counters.read().unwrap();
        for (name, val) in counters.iter() {
            let _ = writeln!(out, "# TYPE {} counter", name);
            let _ = writeln!(out, "{} {}", name, val.load(Ordering::Relaxed));
        }

        let gauges = self.gauges.read().unwrap();
        for (name, val) in gauges.iter() {
            let _ = writeln!(out, "# TYPE {} gauge", name);
            let _ = writeln!(out, "{} {}", name, val.load(Ordering::Relaxed));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();
        registry.inc_counter("orchestra_packets_processed_total", 42);
        assert_eq!(
            registry.get_counter("orchestra_packets_processed_total"),
            42
        );

        registry.set_gauge("orchestra_clusters_active", 3);
        assert_eq!(registry.get_gauge("orchestra_clusters_active"), 3);

        let prom = registry.render_prometheus();
        assert!(prom.contains("orchestra_packets_processed_total 42"));
        assert!(prom.contains("orchestra_clusters_active 3"));
    }
}
