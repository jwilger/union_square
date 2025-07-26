//! AWS SigV4 authentication pass-through for Bedrock
//!
//! This module handles the pass-through of AWS SigV4 authentication headers
//! without storing or processing credentials.

use crate::providers::ProviderError;
use hyper::header::{HeaderMap, HeaderName};

/// Type-safe AWS SigV4 header constants
pub struct SigV4Headers;

impl SigV4Headers {
    pub const AUTHORIZATION: HeaderName = HeaderName::from_static("authorization");
    pub const AMZ_DATE: HeaderName = HeaderName::from_static("x-amz-date");
    pub const AMZ_SECURITY_TOKEN: HeaderName = HeaderName::from_static("x-amz-security-token");
    pub const AMZ_CONTENT_SHA256: HeaderName = HeaderName::from_static("x-amz-content-sha256");
    pub const AMZ_TARGET: HeaderName = HeaderName::from_static("x-amz-target");

    /// Get all required SigV4 headers as a Vec
    pub fn all() -> Vec<HeaderName> {
        vec![
            Self::AUTHORIZATION,
            Self::AMZ_DATE,
            Self::AMZ_SECURITY_TOKEN,
            Self::AMZ_CONTENT_SHA256,
            Self::AMZ_TARGET,
        ]
    }

    /// Prefix for AWS-specific headers
    pub const AMZ_PREFIX: &'static str = "x-amz-";
}

/// Extract and validate SigV4 authentication headers
pub fn extract_sigv4_headers(headers: &HeaderMap) -> Result<HeaderMap, ProviderError> {
    let mut auth_headers = HeaderMap::new();

    // Check for authorization header
    if !headers.contains_key(&SigV4Headers::AUTHORIZATION) {
        return Err(ProviderError::AuthenticationError(
            "Missing AWS SigV4 authorization header".to_string(),
        ));
    }

    // Extract all SigV4 related headers
    for header_name in SigV4Headers::all() {
        if let Some(header_value) = headers.get(&header_name) {
            auth_headers.insert(header_name, header_value.clone());
        }
    }

    // Also pass through any x-amz-* headers
    for (name, value) in headers.iter() {
        if name.as_str().starts_with(SigV4Headers::AMZ_PREFIX) {
            auth_headers.insert(name.clone(), value.clone());
        }
    }

    Ok(auth_headers)
}

/// Validate that required SigV4 headers are present
pub fn validate_sigv4_auth(headers: &HeaderMap) -> Result<(), ProviderError> {
    // Must have authorization header
    if !headers.contains_key(&SigV4Headers::AUTHORIZATION) {
        return Err(ProviderError::AuthenticationError(
            "Missing authorization header".to_string(),
        ));
    }

    // Must have x-amz-date header
    if !headers.contains_key(&SigV4Headers::AMZ_DATE) {
        return Err(ProviderError::AuthenticationError(
            "Missing x-amz-date header".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::HeaderValue;

    #[test]
    fn test_extract_sigv4_headers_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("AWS4-HMAC-SHA256..."),
        );
        headers.insert("x-amz-date", HeaderValue::from_static("20250126T120000Z"));
        headers.insert("x-amz-security-token", HeaderValue::from_static("token"));
        headers.insert("x-amz-custom", HeaderValue::from_static("custom"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let auth_headers = extract_sigv4_headers(&headers).unwrap();

        // Should include all SigV4 headers
        assert!(auth_headers.contains_key(&SigV4Headers::AUTHORIZATION));
        assert!(auth_headers.contains_key(&SigV4Headers::AMZ_DATE));
        assert!(auth_headers.contains_key(&SigV4Headers::AMZ_SECURITY_TOKEN));
        assert!(auth_headers.contains_key("x-amz-custom"));

        // Should not include non-SigV4 headers
        assert!(!auth_headers.contains_key("content-type"));
    }

    #[test]
    fn test_extract_sigv4_headers_missing_auth() {
        let headers = HeaderMap::new();

        let result = extract_sigv4_headers(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing AWS SigV4"));
    }

    #[test]
    fn test_validate_sigv4_auth_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("AWS4-HMAC-SHA256..."),
        );
        headers.insert("x-amz-date", HeaderValue::from_static("20250126T120000Z"));

        assert!(validate_sigv4_auth(&headers).is_ok());
    }

    #[test]
    fn test_validate_sigv4_auth_missing_date() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("AWS4-HMAC-SHA256..."),
        );

        let result = validate_sigv4_auth(&headers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("x-amz-date"));
    }
}
