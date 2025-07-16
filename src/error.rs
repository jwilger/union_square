use thiserror::Error;

/// Union Square application error types
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("EventCore error: {0}")]
    EventCore(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Application error: {message}")]
    Application { message: String },

    #[error("Invalid input: {field}")]
    InvalidInput { field: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal server error")]
    Internal,
}

impl Error {
    pub fn application(message: impl Into<String>) -> Self {
        Self::Application {
            message: message.into(),
        }
    }

    pub fn invalid_input(field: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
