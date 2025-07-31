//! Domain entity identifiers for event-sourced entities
//!
//! This module provides type-safe identifiers for domain entities that
//! participate in event sourcing. Each identifier type is a newtype around
//! UUID v7, providing time-ordered generation suitable for event stores.

use nutype::nutype;
use uuid::Uuid;

/// Unique identifier for an analysis process
#[nutype(derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    AsRef
))]
pub struct AnalysisId(Uuid);

impl AnalysisId {
    pub fn generate() -> Self {
        // Uuid::now_v7() generates a time-ordered UUID suitable for event sourcing
        Self::new(Uuid::now_v7())
    }
}

impl Default for AnalysisId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Unique identifier for a test case extraction
#[nutype(derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    AsRef
))]
pub struct ExtractionId(Uuid);

impl ExtractionId {
    pub fn generate() -> Self {
        // Uuid::now_v7() generates a time-ordered UUID suitable for event sourcing
        Self::new(Uuid::now_v7())
    }
}

impl Default for ExtractionId {
    fn default() -> Self {
        Self::generate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_id_generation_is_unique() {
        let id1 = AnalysisId::generate();
        let id2 = AnalysisId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn extraction_id_generation_is_unique() {
        let id1 = ExtractionId::generate();
        let id2 = ExtractionId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn ids_are_time_ordered() {
        let id1 = AnalysisId::generate();
        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = AnalysisId::generate();

        // UUIDv7 are time-ordered, so id2 should be greater than id1
        assert!(id2.as_ref().as_bytes() > id1.as_ref().as_bytes());
    }
}
