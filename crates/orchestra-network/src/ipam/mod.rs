//! IP Address Management (IPAM) for cluster bridges and nodes.

use ipnet::Ipv4Net;
use orchestra_core::error::NetworkError;
use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Thread-safe IP address pool allocator for a subnet.
#[derive(Debug, Clone)]
pub struct IpamPool {
    subnet: Ipv4Net,
    gateway: Ipv4Addr,
    allocated: Arc<Mutex<HashSet<Ipv4Addr>>>,
}

impl IpamPool {
    /// Creates a new IPAM pool for the given CIDR subnet string.
    pub fn new(cidr: &str) -> Result<Self, NetworkError> {
        let subnet = Ipv4Net::from_str(cidr)
            .map_err(|e| NetworkError::InvalidCidr(format!("{cidr}: {e}")))?;

        // By convention, gateway is the first usable host in the subnet
        let gateway = subnet.hosts().next().ok_or_else(|| {
            NetworkError::IpAllocationFailed("Subnet has no usable hosts".to_string())
        })?;

        let mut allocated = HashSet::new();
        allocated.insert(gateway); // Reserve gateway

        Ok(Self {
            subnet,
            gateway,
            allocated: Arc::new(Mutex::new(allocated)),
        })
    }

    /// Returns the gateway IP address for this subnet.
    pub fn gateway(&self) -> Ipv4Addr {
        self.gateway
    }

    /// Returns the configured subnet.
    pub fn subnet(&self) -> Ipv4Net {
        self.subnet
    }

    /// Allocates the next available IP address in the subnet.
    pub fn allocate(&self) -> Result<Ipv4Addr, NetworkError> {
        let mut allocated = self.allocated.lock().unwrap();

        for host in self.subnet.hosts() {
            if !allocated.contains(&host) {
                allocated.insert(host);
                debug!(ip = %host, "Allocated IP address from pool");
                return Ok(host);
            }
        }

        Err(NetworkError::IpAllocationFailed(format!(
            "Subnet {} is exhausted",
            self.subnet
        )))
    }

    /// Releases a previously allocated IP address back to the pool.
    pub fn release(&self, ip: Ipv4Addr) {
        if ip != self.gateway {
            let mut allocated = self.allocated.lock().unwrap();
            allocated.remove(&ip);
            debug!(ip = %ip, "Released IP address back to pool");
        }
    }

    /// Checks if an IP is currently allocated.
    pub fn is_allocated(&self, ip: Ipv4Addr) -> bool {
        let allocated = self.allocated.lock().unwrap();
        allocated.contains(&ip)
    }

    /// Returns the number of currently allocated IPs.
    pub fn allocated_count(&self) -> usize {
        let allocated = self.allocated.lock().unwrap();
        allocated.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipam_pool_allocation_and_release() {
        let pool = IpamPool::new("172.24.0.0/24").unwrap();
        assert_eq!(pool.gateway(), Ipv4Addr::new(172, 24, 0, 1));
        assert_eq!(pool.allocated_count(), 1); // gateway

        let ip1 = pool.allocate().unwrap();
        assert_eq!(ip1, Ipv4Addr::new(172, 24, 0, 2));

        let ip2 = pool.allocate().unwrap();
        assert_eq!(ip2, Ipv4Addr::new(172, 24, 0, 3));
        assert_eq!(pool.allocated_count(), 3);

        pool.release(ip1);
        assert_eq!(pool.allocated_count(), 2);

        // Next allocation should reuse ip1
        let ip3 = pool.allocate().unwrap();
        assert_eq!(ip3, ip1);
    }

    #[test]
    fn test_ipam_invalid_cidr() {
        assert!(IpamPool::new("not-a-cidr").is_err());
    }
}
