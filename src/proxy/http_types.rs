//! Type-safe HTTP components for the proxy service
//!
//! This module provides strongly-typed wrappers for HTTP concepts,
//! ensuring compile-time safety and preventing common errors.

use nutype::nutype;
use std::fmt;

/// A validated HTTP path component
#[nutype(
    derive(Clone, Debug, Display, Hash, PartialEq, Eq, TryFrom, AsRef),
    validate(predicate = |s: &str| s.starts_with('/')),
)]
pub struct HttpPath(String);

impl HttpPath {
    /// Extract the path from a URI
    pub fn from_uri(uri: &hyper::Uri) -> Self {
        // URI path is always valid and starts with /
        // We can safely unwrap because URI paths always start with /
        Self::try_new(uri.path().to_string()).expect("URI path should always be valid")
    }

    /// Check if this path matches a pattern (simple prefix matching)
    pub fn matches(&self, pattern: &str) -> bool {
        self.as_ref().starts_with(pattern)
    }

    /// Get the path segments
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.as_ref()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
    }
}

/// A validated query string
#[nutype(
    derive(Clone, Debug, Display, PartialEq, Eq, TryFrom, AsRef),
    validate(predicate = |s: &str| !s.contains('#')), // No fragments in query
)]
pub struct QueryString(String);

impl QueryString {
    /// Create an empty query string
    pub fn empty() -> Self {
        // Empty string is valid (no fragments)
        Self::try_new(String::new()).expect("Empty query string should be valid")
    }

    /// Parse query parameters
    pub fn parse_params(&self) -> Vec<(String, String)> {
        if self.as_ref().is_empty() {
            return Vec::new();
        }

        self.as_ref()
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((
                        urlencoding::decode(key).unwrap_or_default().into_owned(),
                        urlencoding::decode(value).unwrap_or_default().into_owned(),
                    )),
                    (Some(key), None) => Some((
                        urlencoding::decode(key).unwrap_or_default().into_owned(),
                        String::new(),
                    )),
                    _ => None,
                }
            })
            .collect()
    }
}

/// A complete path and query component
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathAndQuery {
    path: HttpPath,
    query: Option<QueryString>,
}

impl PathAndQuery {
    /// Create from path and optional query
    pub fn new(path: HttpPath, query: Option<QueryString>) -> Self {
        Self { path, query }
    }

    /// Extract from a URI
    pub fn from_uri(uri: &hyper::Uri) -> Self {
        let path = HttpPath::from_uri(uri);
        let query = uri
            .query()
            .and_then(|q| QueryString::try_new(q.to_string()).ok());
        Self { path, query }
    }

    /// Get the path component
    pub fn path(&self) -> &HttpPath {
        &self.path
    }

    /// Get the query component
    pub fn query(&self) -> Option<&QueryString> {
        self.query.as_ref()
    }

    /// Convert to string representation
    pub fn as_str(&self) -> String {
        match &self.query {
            Some(q) if !q.as_ref().is_empty() => format!("{}?{}", self.path, q),
            _ => self.path.to_string(),
        }
    }
}

impl fmt::Display for PathAndQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Safe HTTP method wrapper with validation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SafeHttpMethod(http::Method);

impl SafeHttpMethod {
    /// Standard HTTP methods as constants
    pub const GET: Self = Self(http::Method::GET);
    pub const POST: Self = Self(http::Method::POST);
    pub const PUT: Self = Self(http::Method::PUT);
    pub const DELETE: Self = Self(http::Method::DELETE);
    pub const HEAD: Self = Self(http::Method::HEAD);
    pub const OPTIONS: Self = Self(http::Method::OPTIONS);
    pub const CONNECT: Self = Self(http::Method::CONNECT);
    pub const PATCH: Self = Self(http::Method::PATCH);
    pub const TRACE: Self = Self(http::Method::TRACE);

    /// Create from http::Method
    pub fn from_method(method: http::Method) -> Self {
        Self(method)
    }

    /// Get the inner method
    pub fn as_method(&self) -> &http::Method {
        &self.0
    }

    /// Check if this is a safe method (no side effects)
    pub fn is_safe(&self) -> bool {
        matches!(
            self.0,
            http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
        )
    }

    /// Check if this method typically has a body
    pub fn has_body(&self) -> bool {
        matches!(
            self.0,
            http::Method::POST | http::Method::PUT | http::Method::PATCH
        )
    }
}

impl fmt::Display for SafeHttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<http::Method> for SafeHttpMethod {
    fn from(method: http::Method) -> Self {
        Self(method)
    }
}

/// Safe status code wrapper with semantic helpers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SafeStatusCode(http::StatusCode);

impl SafeStatusCode {
    /// Create from http::StatusCode
    pub fn from_status(status: http::StatusCode) -> Self {
        Self(status)
    }

    /// Get the inner status code
    pub fn as_status(&self) -> http::StatusCode {
        self.0
    }

    /// Check if this is a success status (2xx)
    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    /// Check if this is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    /// Check if this is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    /// Check if this status indicates the request can be retried
    pub fn is_retriable(&self) -> bool {
        matches!(self.0.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
    }
}

impl fmt::Display for SafeStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<http::StatusCode> for SafeStatusCode {
    fn from(status: http::StatusCode) -> Self {
        Self(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_path_validation() {
        assert!(HttpPath::try_new("/valid/path".to_string()).is_ok());
        assert!(HttpPath::try_new("invalid".to_string()).is_err());
        assert!(HttpPath::try_new("".to_string()).is_err());
    }

    #[test]
    fn test_http_path_segments() {
        let path = HttpPath::try_new("/api/v1/users".to_string()).unwrap();
        let segments: Vec<_> = path.segments().collect();
        assert_eq!(segments, vec!["api", "v1", "users"]);
    }

    #[test]
    fn test_http_path_matching() {
        let path = HttpPath::try_new("/health/check".to_string()).unwrap();
        assert!(path.matches("/health"));
        assert!(!path.matches("/metrics"));
    }

    #[test]
    fn test_query_string_parsing() {
        let query = QueryString::try_new("foo=bar&baz=qux".to_string()).unwrap();
        let params = query.parse_params();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("foo".to_string(), "bar".to_string()));
        assert_eq!(params[1], ("baz".to_string(), "qux".to_string()));
    }

    #[test]
    fn test_path_and_query() {
        let path = HttpPath::try_new("/api/test".to_string()).unwrap();
        let query = QueryString::try_new("key=value".to_string()).unwrap();
        let pq = PathAndQuery::new(path, Some(query));
        assert_eq!(pq.as_str(), "/api/test?key=value");
    }

    #[test]
    fn test_safe_http_method() {
        assert!(SafeHttpMethod::GET.is_safe());
        assert!(!SafeHttpMethod::POST.is_safe());
        assert!(SafeHttpMethod::POST.has_body());
        assert!(!SafeHttpMethod::GET.has_body());
    }

    #[test]
    fn test_safe_status_code() {
        let ok = SafeStatusCode::from_status(http::StatusCode::OK);
        assert!(ok.is_success());
        assert!(!ok.is_client_error());

        let not_found = SafeStatusCode::from_status(http::StatusCode::NOT_FOUND);
        assert!(not_found.is_client_error());

        let gateway_timeout = SafeStatusCode::from_status(http::StatusCode::GATEWAY_TIMEOUT);
        assert!(gateway_timeout.is_retriable());
    }
}
