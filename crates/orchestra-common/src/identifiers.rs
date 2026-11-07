//! Strongly typed domain identifiers for Orchestra.
//!
//! Provides newtype wrappers around unique identifiers (`ClusterId`, `NodeId`,
//! `NetworkId`, `OperationId`) ensuring type safety across cluster boundaries.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $prefix:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier from a string without prefix validation.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// Generates a new random unique identifier with prefix.
            pub fn generate() -> Self {
                let uuid = Uuid::new_v4();
                Self(format!("{}-{}", $prefix, &uuid.simple().to_string()[..12]))
            }

            /// Generates a new random unique identifier using full UUID v4.
            pub fn generate_uuid() -> Self {
                Self(format!("{}-{}", $prefix, Uuid::new_v4()))
            }

            /// Returns the identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the identifier and returns the inner `String`.
            pub fn into_inner(self) -> String {
                self.0
            }

            /// Returns the expected prefix for this identifier type.
            pub const fn prefix() -> &'static str {
                $prefix
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl FromStr for $name {
            type Err = std::convert::Infallible;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }
    };
}

define_id!(
    ClusterId,
    "clus",
    "Globally unique identifier for a Kubernetes cluster managed by Orchestra."
);

define_id!(
    NodeId,
    "node",
    "Globally unique identifier for a cluster node container."
);

define_id!(
    NetworkId,
    "net",
    "Globally unique identifier for an Orchestra bridge or overlay network."
);

define_id!(
    OperationId,
    "op",
    "Globally unique identifier for an asynchronous multi-stage operation."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_generation() {
        let cluster_id = ClusterId::generate();
        assert!(cluster_id.as_str().starts_with("clus-"));
        assert_eq!(ClusterId::prefix(), "clus");

        let node_id = NodeId::generate();
        assert!(node_id.as_str().starts_with("node-"));
        assert_eq!(NodeId::prefix(), "node");

        let net_id = NetworkId::generate();
        assert!(net_id.as_str().starts_with("net-"));

        let op_id = OperationId::generate();
        assert!(op_id.as_str().starts_with("op-"));
    }

    #[test]
    fn test_identifier_conversions() {
        let id_str = "clus-custom-test-123";
        let cluster_id = ClusterId::from(id_str);
        assert_eq!(cluster_id.as_str(), id_str);
        assert_eq!(cluster_id.to_string(), id_str);
        assert_eq!(&*cluster_id, id_str);

        let parsed: ClusterId = id_str.parse().unwrap();
        assert_eq!(parsed, cluster_id);

        let inner = cluster_id.into_inner();
        assert_eq!(inner, id_str);
    }

    #[test]
    fn test_identifier_serde_roundtrip() {
        let original = NodeId::generate();
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, format!("\"{}\"", original.as_str()));

        let deserialized: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }
}
