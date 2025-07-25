//! Target URL resolution and path handling for proxy requests

use crate::proxy::headers::{paths, X_TARGET_URL};
use crate::proxy::types::*;
use hyper::{Request, Uri};

/// Strategy for resolving target URLs and handling path composition
pub struct UrlResolver;

impl UrlResolver {
    /// Extract target URL from request headers
    pub fn extract_target_url<B>(request: &Request<B>) -> ProxyResult<TargetUrl> {
        let target_url_str = request
            .headers()
            .get(X_TARGET_URL)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                ProxyError::InvalidTargetUrl("Missing X-Target-Url header".to_string())
            })?;

        TargetUrl::try_new(target_url_str.to_string()).map_err(|e| {
            ProxyError::InvalidTargetUrl(format!("Invalid target URL '{target_url_str}': {e}"))
        })
    }

    /// Resolve the final URI for the outgoing request
    ///
    /// This handles the logic for combining the target URL with the original request path:
    /// - If target URL has a path, use it as-is
    /// - If target URL is just the base, append the original request path
    pub fn resolve_target_uri(target_url: &TargetUrl, original_uri: &Uri) -> ProxyResult<Uri> {
        // Parse the target URL to understand its structure
        let target_uri: Uri = target_url
            .as_ref()
            .parse()
            .map_err(|_| ProxyError::InvalidTargetUrl(target_url.as_ref().to_string()))?;

        // Determine the final URI based on target URL structure
        let final_uri_str = if target_uri.path() != "/" && !target_uri.path().is_empty() {
            // Target URL already has a path, use it directly
            target_url.as_ref().to_string()
        } else {
            // Target URL is just the base, append the original path
            let path_and_query = original_uri
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or(paths::DEFAULT);

            format!(
                "{}{}",
                target_url.as_ref().trim_end_matches('/'),
                path_and_query
            )
        };

        // Parse and validate the final URI
        final_uri_str
            .parse()
            .map_err(|_| ProxyError::InvalidTargetUrl(final_uri_str))
    }

    /// Validate that a target URL is reachable (placeholder for future health checking)
    pub fn validate_target_url(_target_url: &TargetUrl) -> ProxyResult<()> {
        // Future implementation could include:
        // - DNS resolution check
        // - Basic connectivity test
        // - Circuit breaker pattern
        // - Rate limiting per target
        Ok(())
    }
}

/// Configuration for URL resolution behavior
#[derive(Clone, Debug)]
pub struct UrlResolverConfig {
    /// Whether to perform health checks on target URLs
    pub enable_health_checks: bool,
    /// Maximum allowed target URL length
    pub max_url_length: usize,
    /// Allowed target URL schemes
    pub allowed_schemes: Vec<String>,
}

impl Default for UrlResolverConfig {
    fn default() -> Self {
        Self {
            enable_health_checks: false,
            max_url_length: 2048,
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Empty;
    use hyper::Request;

    #[test]
    fn test_extract_target_url_success() {
        let request = Request::builder()
            .header(X_TARGET_URL, "https://api.example.com")
            .body(Empty::<bytes::Bytes>::new())
            .unwrap();

        let target_url = UrlResolver::extract_target_url(&request).unwrap();
        assert_eq!(target_url.as_ref(), "https://api.example.com");
    }

    #[test]
    fn test_extract_target_url_missing_header() {
        let request = Request::builder()
            .body(Empty::<bytes::Bytes>::new())
            .unwrap();

        let result = UrlResolver::extract_target_url(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing X-Target-Url header"));
    }

    #[test]
    fn test_extract_target_url_invalid_url() {
        let request = Request::builder()
            .header(X_TARGET_URL, "not-a-url")
            .body(Empty::<bytes::Bytes>::new())
            .unwrap();

        let result = UrlResolver::extract_target_url(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid target URL"));
    }

    #[test]
    fn test_resolve_target_uri_base_url_with_path() {
        let target_url = TargetUrl::try_new("https://api.example.com".to_string()).unwrap();
        let original_uri: Uri = "/users/123?param=value".parse().unwrap();

        let resolved = UrlResolver::resolve_target_uri(&target_url, &original_uri).unwrap();
        assert_eq!(
            resolved.to_string(),
            "https://api.example.com/users/123?param=value"
        );
    }

    #[test]
    fn test_resolve_target_uri_with_existing_path() {
        let target_url =
            TargetUrl::try_new("https://api.example.com/v1/endpoint".to_string()).unwrap();
        let original_uri: Uri = "/users/123?param=value".parse().unwrap();

        let resolved = UrlResolver::resolve_target_uri(&target_url, &original_uri).unwrap();
        // When target URL has a path, use it as-is (ignore original path)
        assert_eq!(resolved.to_string(), "https://api.example.com/v1/endpoint");
    }

    #[test]
    fn test_resolve_target_uri_with_trailing_slash() {
        let target_url = TargetUrl::try_new("https://api.example.com/".to_string()).unwrap();
        let original_uri: Uri = "/users/123".parse().unwrap();

        let resolved = UrlResolver::resolve_target_uri(&target_url, &original_uri).unwrap();
        assert_eq!(resolved.to_string(), "https://api.example.com/users/123");
    }

    #[test]
    fn test_resolve_target_uri_root_path() {
        let target_url = TargetUrl::try_new("https://api.example.com".to_string()).unwrap();
        let original_uri: Uri = "/".parse().unwrap();

        let resolved = UrlResolver::resolve_target_uri(&target_url, &original_uri).unwrap();
        assert_eq!(resolved.to_string(), "https://api.example.com/");
    }

    #[test]
    fn test_validate_target_url() {
        let target_url = TargetUrl::try_new("https://api.example.com".to_string()).unwrap();

        // Current implementation always succeeds
        assert!(UrlResolver::validate_target_url(&target_url).is_ok());
    }

    #[test]
    fn test_url_resolver_config_default() {
        let config = UrlResolverConfig::default();
        assert!(!config.enable_health_checks);
        assert_eq!(config.max_url_length, 2048);
        assert_eq!(config.allowed_schemes, vec!["http", "https"]);
    }
}
