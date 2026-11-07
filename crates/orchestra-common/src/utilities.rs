//! General utilities: retry backoff, CIDR helpers, port checks, and DNS/Kubernetes name validators.

use ipnet::IpNet;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::time::Duration;
use tracing::warn;

/// Configuration for exponential backoff retries.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Initial backoff duration.
    pub initial_delay: Duration,
    /// Maximum backoff duration.
    pub max_delay: Duration,
    /// Multiplier per retry attempt.
    pub factor: f64,
    /// Maximum number of attempts (including initial attempt).
    pub max_attempts: usize,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            factor: 2.0,
            max_attempts: 5,
        }
    }
}

/// Executes an async operation with exponential backoff.
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut current_delay = config.initial_delay;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(val) => return Ok(val),
            Err(err) => {
                if attempt == config.max_attempts {
                    return Err(err);
                }
                warn!(
                    operation = %operation_name,
                    attempt = attempt,
                    max_attempts = config.max_attempts,
                    delay_ms = current_delay.as_millis(),
                    error = %err,
                    "Operation failed, retrying after backoff"
                );
                tokio::time::sleep(current_delay).await;
                current_delay = Duration::from_secs_f64(
                    (current_delay.as_secs_f64() * config.factor)
                        .min(config.max_delay.as_secs_f64()),
                );
            }
        }
    }

    unreachable!()
}

/// Parses and validates an IPv4 or IPv6 CIDR block.
pub fn parse_cidr(cidr: &str) -> Result<IpNet, String> {
    IpNet::from_str(cidr).map_err(|e| format!("Invalid CIDR '{}': {}", cidr, e))
}

/// Checks whether an IP address belongs to a CIDR network block.
pub fn cidr_contains_ip(cidr: &IpNet, ip: &IpAddr) -> bool {
    cidr.contains(ip)
}

/// Validates that two CIDR blocks do not overlap.
pub fn cidrs_do_not_overlap(a: &IpNet, b: &IpNet) -> bool {
    !a.contains(b) && !b.contains(a)
}

/// Checks whether a port is within the valid TCP/UDP port range (1..=65535).
pub fn is_valid_port(port: u32) -> bool {
    (1..=65535).contains(&port)
}

/// Checks whether a port is within standard Kubernetes NodePort range (30000..=32767).
pub fn is_nodeport(port: u16) -> bool {
    (30000..=32767).contains(&port)
}

/// Validates whether a string is a valid RFC 1123 DNS subdomain name.
/// (lowercase alphanumeric, '-', must start and end with alphanumeric, max 63 characters).
pub fn is_valid_dns_subdomain(name: &str) -> bool {
    if name.is_empty() || name.len() > 63 {
        return false;
    }

    let bytes = name.as_bytes();
    if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validates a Kubernetes label key.
/// Max 63 characters (excluding prefix), alphanumeric or [-_.] characters, starting and ending with alphanumeric.
pub fn is_valid_label_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    let (prefix, name) = if let Some((p, n)) = key.split_once('/') {
        if p.is_empty() || p.len() > 253 || !p.split('.').all(is_valid_dns_subdomain) {
            return false;
        }
        (Some(p), n)
    } else {
        (None, key)
    };

    let _ = prefix;
    if name.is_empty() || name.len() > 63 {
        return false;
    }

    let bytes = name.as_bytes();
    if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Validates a Kubernetes label value (can be empty, max 63 chars, alphanumeric or [-_.]).
pub fn is_valid_label_value(val: &str) -> bool {
    if val.is_empty() {
        return true;
    }
    if val.len() > 63 {
        return false;
    }

    let bytes = val.as_bytes();
    if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return false;
    }

    val.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Sanitizes a string to make it a compliant RFC 1123 DNS subdomain.
pub fn sanitize_name(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for c in input.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            sanitized.push(lower);
            last_was_dash = false;
        } else if !last_was_dash && !sanitized.is_empty() {
            sanitized.push('-');
            last_was_dash = true;
        }
    }

    // Trim trailing dashes
    while sanitized.ends_with('-') {
        sanitized.pop();
    }

    // Limit to 63 chars
    if sanitized.len() > 63 {
        sanitized.truncate(63);
        while sanitized.ends_with('-') {
            sanitized.pop();
        }
    }

    if sanitized.is_empty() {
        "cluster".to_string()
    } else {
        sanitized
    }
}

/// Increments an IPv4 address by a given offset.
pub fn add_ip_offset(ip: Ipv4Addr, offset: u32) -> Ipv4Addr {
    let raw = u32::from(ip);
    Ipv4Addr::from(raw.saturating_add(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_validation() {
        assert!(is_valid_dns_subdomain("my-cluster"));
        assert!(is_valid_dns_subdomain("worker-node-1"));
        assert!(is_valid_dns_subdomain("k8s"));
        assert!(!is_valid_dns_subdomain("-invalid"));
        assert!(!is_valid_dns_subdomain("invalid-"));
        assert!(!is_valid_dns_subdomain("UPPERCASE"));
        assert!(!is_valid_dns_subdomain("with_underscore"));
        assert!(!is_valid_dns_subdomain(&"a".repeat(64)));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("My Cluster!"), "my-cluster");
        assert_eq!(sanitize_name("___test___"), "test");
        assert_eq!(sanitize_name(""), "cluster");
        assert_eq!(sanitize_name("dev.local/k8s"), "dev-local-k8s");
    }

    #[test]
    fn test_cidr_utilities() {
        let net_a = parse_cidr("10.244.0.0/16").unwrap();
        let net_b = parse_cidr("10.96.0.0/12").unwrap();
        assert!(cidrs_do_not_overlap(&net_a, &net_b));

        let ip_in_a: IpAddr = "10.244.1.5".parse().unwrap();
        let ip_out_a: IpAddr = "10.96.0.1".parse().unwrap();
        assert!(cidr_contains_ip(&net_a, &ip_in_a));
        assert!(!cidr_contains_ip(&net_a, &ip_out_a));
    }

    #[test]
    fn test_port_checks() {
        assert!(is_valid_port(80));
        assert!(is_valid_port(6443));
        assert!(!is_valid_port(0));
        assert!(!is_valid_port(70000));

        assert!(is_nodeport(30000));
        assert!(is_nodeport(32767));
        assert!(!is_nodeport(8080));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            factor: 2.0,
            max_attempts: 3,
        };

        let mut attempts = 0;
        let result = retry_with_backoff(&config, "test_op", || {
            attempts += 1;
            async move {
                if attempts < 2 {
                    Err("transient error")
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2);
    }
}
