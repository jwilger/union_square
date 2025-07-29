use crate::domain::types::{ErrorMessage, FieldName, ResourceId};
use crate::providers::constants::error_messages;
use thiserror::Error;

/// Union Square application error types
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("EventCore error: {0}")]
    EventCore(ErrorMessage),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Application error: {message}")]
    Application { message: ErrorMessage },

    #[error("Invalid input: {field}")]
    InvalidInput { field: FieldName },

    #[error("Not found: {resource}")]
    NotFound { resource: ResourceId },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal server error")]
    Internal,

    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
}

impl Error {
    pub fn application(message: impl Into<String>) -> Self {
        Self::Application {
            message: ErrorMessage::try_new(message.into()).unwrap_or_else(|_| {
                ErrorMessage::try_new(error_messages::INVALID_ERROR_MESSAGE.to_string()).unwrap()
            }),
        }
    }

    pub fn invalid_input(field: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: FieldName::try_new(field.into()).unwrap_or_else(|_| {
                FieldName::try_new(error_messages::UNKNOWN_FIELD.to_string()).unwrap()
            }),
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: ResourceId::try_new(resource.into()).unwrap_or_else(|_| {
                ResourceId::try_new(error_messages::UNKNOWN_RESOURCE.to_string()).unwrap()
            }),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
