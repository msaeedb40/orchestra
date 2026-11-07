//! Time utilities, timestamp formatting, and human-readable durations.

use chrono::{DateTime, Utc};
use std::fmt;
use std::time::{Duration, Instant};

/// Returns the current UTC timestamp.
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Returns the current Unix timestamp in seconds.
pub fn now_epoch_secs() -> i64 {
    Utc::now().timestamp()
}

/// Returns the current Unix timestamp in milliseconds.
pub fn now_epoch_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Formats a `DateTime<Utc>` into an RFC 3339 formatted string.
pub fn format_rfc3339(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Parses an RFC 3339 formatted timestamp string.
pub fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Formats a `Duration` into a clean human-readable string.
/// Examples: "450ms", "12s", "2m 31s", "4h 15m", "2d 4h"
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs == 0 {
        if millis == 0 {
            let micros = duration.subsec_micros();
            if micros == 0 {
                return format!("{}ns", duration.subsec_nanos());
            }
            return format!("{}µs", micros);
        }
        return format!("{}ms", millis);
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if days > 0 {
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if hours > 0 {
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        if secs > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}m", minutes)
        }
    } else {
        format!("{}s", secs)
    }
}

/// Formats the age of a past timestamp relative to current time.
pub fn format_age(timestamp: &DateTime<Utc>) -> String {
    let now = Utc::now();
    if timestamp > &now {
        return "in the future".to_string();
    }
    let duration = (now - *timestamp).to_std().unwrap_or(Duration::ZERO);
    format_duration(duration)
}

/// High-resolution stopwatch for measuring operation durations.
#[derive(Debug, Clone)]
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    /// Starts a new stopwatch timer.
    pub fn start_new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Returns the elapsed `Duration`.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Formats the elapsed time as a human-readable string.
    pub fn elapsed_human(&self) -> String {
        format_duration(self.elapsed())
    }

    /// Resets the stopwatch to current instant.
    pub fn restart(&mut self) -> Duration {
        let elapsed = self.elapsed();
        self.start = Instant::now();
        elapsed
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::start_new()
    }
}

impl fmt::Display for Stopwatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.elapsed_human())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(150)), "150ms");
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m");
        assert_eq!(format_duration(Duration::from_secs(90000)), "1d 1h");
    }

    #[test]
    fn test_stopwatch() {
        let mut sw = Stopwatch::start_new();
        std::thread::sleep(Duration::from_millis(5));
        assert!(sw.elapsed() >= Duration::from_millis(4));
        let reset_elapsed = sw.restart();
        assert!(reset_elapsed >= Duration::from_millis(4));
    }

    #[test]
    fn test_rfc3339_roundtrip() {
        let now = now_utc();
        let formatted = format_rfc3339(&now);
        let parsed = parse_rfc3339(&formatted).unwrap();
        assert_eq!(now.timestamp(), parsed.timestamp());
    }
}
