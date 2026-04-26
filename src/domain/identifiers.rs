//! Domain identifiers for analysis and extraction workflows.

use nutype::nutype;
use uuid::Uuid;

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
        Self::new(Uuid::now_v7())
    }
}

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
        Self::new(Uuid::now_v7())
    }
}
