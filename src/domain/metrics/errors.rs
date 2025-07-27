//! Error types for metrics calculations

use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors that can occur during metrics calculations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricsError {
    /// Invalid F-score value (outside 0.0-1.0 range)
    InvalidValue(f64),
    /// Invalid precision value
    InvalidPrecision(String),
    /// Invalid recall value
    InvalidRecall(String),
    /// Invalid beta parameter
    InvalidBeta(String),
    /// Calculation error
    CalculationError(String),
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::InvalidValue(value) => {
                write!(
                    f,
                    "Invalid F-score value: {value} (must be between 0.0 and 1.0)"
                )
            }
            MetricsError::InvalidPrecision(msg) => write!(f, "Invalid precision: {msg}"),
            MetricsError::InvalidRecall(msg) => write!(f, "Invalid recall: {msg}"),
            MetricsError::InvalidBeta(msg) => write!(f, "Invalid beta parameter: {msg}"),
            MetricsError::CalculationError(msg) => write!(f, "F-score calculation error: {msg}"),
        }
    }
}

impl std::error::Error for MetricsError {}
