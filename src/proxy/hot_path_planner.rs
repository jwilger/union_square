//! Pure planning functions for the hot path
//!
//! This module extracts pure, side-effect-free planning from the imperative
//! hot-path shell. Boundary conversions (HTTP types to domain semantic types)
//! happen here before facts enter the audit workflow.
//!
//! All functions in this module are deterministic for the same inputs and
//! require no IO, Tokio runtime, or HTTP client.

use crate::proxy::types::*;

/// Planned audit facts for an incoming request.
///
/// This is the result of boundary parsing and planning; the imperative shell
/// decides how to persist it.
#[derive(Debug, Clone)]
pub enum PlannedRequestAudit {
    /// Successfully parsed request metadata.
    Received {
        method: HttpMethod,
        uri: RequestUri,
        headers: Headers,
        body_size: BodySize,
    },
    /// Boundary parsing failed; record the failure fact.
    ParseFailed { error: String, phase: ErrorPhase },
}

/// Planned audit facts for a received response.
#[derive(Debug, Clone)]
pub enum PlannedResponseAudit {
    /// Successfully parsed response metadata.
    Received {
        status: HttpStatusCode,
        headers: Headers,
        body_size: BodySize,
        duration_ms: DurationMillis,
    },
    /// Boundary parsing failed; record the failure fact.
    ParseFailed { error: String, phase: ErrorPhase },
}

/// Plan request audit from parsed boundary values.
///
/// This function is pure: it only constructs semantic facts from already-parsed
/// boundary data. No IO, clocks, or mutable state is used.
pub fn plan_request_audit(
    method: Result<HttpMethod, String>,
    uri: Result<RequestUri, String>,
    headers: Vec<(String, String)>,
    body_size: BodySize,
) -> PlannedRequestAudit {
    match (method, uri) {
        (Ok(method), Ok(uri)) => PlannedRequestAudit::Received {
            method,
            uri,
            headers: Headers::from_vec(headers).unwrap_or_default(),
            body_size,
        },
        (Err(method_err), _) => PlannedRequestAudit::ParseFailed {
            error: method_err,
            phase: ErrorPhase::RequestParsing,
        },
        (_, Err(uri_err)) => PlannedRequestAudit::ParseFailed {
            error: uri_err,
            phase: ErrorPhase::RequestParsing,
        },
    }
}

/// Plan response audit from parsed boundary values.
///
/// This function is pure: it only constructs semantic facts from already-parsed
/// boundary data. No IO, clocks, or mutable state is used.
pub fn plan_response_audit(
    status: Result<HttpStatusCode, u16>,
    headers: Vec<(String, String)>,
    body_size: BodySize,
    duration_ms: DurationMillis,
) -> PlannedResponseAudit {
    match status {
        Ok(status) => PlannedResponseAudit::Received {
            status,
            headers: Headers::from_vec(headers).unwrap_or_default(),
            body_size,
            duration_ms,
        },
        Err(invalid_status) => PlannedResponseAudit::ParseFailed {
            error: format!("Invalid HTTP status code '{invalid_status}' received from upstream"),
            phase: ErrorPhase::ResponseReceiving,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_request_audit_with_valid_method_and_uri() {
        let method: Result<HttpMethod, String> =
            Ok(HttpMethod::try_new("POST".to_string()).unwrap());
        let uri: Result<RequestUri, String> =
            Ok(RequestUri::try_new("/v1/chat".to_string()).unwrap());
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        let planned = plan_request_audit(method, uri, headers.clone(), BodySize::from(42));

        assert!(
            matches!(
                planned,
                PlannedRequestAudit::Received {
                    method: _,
                    uri: _,
                    headers: _,
                    body_size
                } if *body_size.as_ref() == 42
            ),
            "Expected Received variant with body_size=42, got {:?}",
            planned
        );
    }

    #[test]
    fn plan_request_audit_with_invalid_method() {
        let method: Result<HttpMethod, String> = Err("empty method".to_string());
        let uri = RequestUri::try_new("/v1/chat".to_string()).map_err(|e| e.to_string());
        let headers = vec![];

        let planned = plan_request_audit(method, uri, headers, BodySize::from(0));

        assert!(
            matches!(
                planned,
                PlannedRequestAudit::ParseFailed {
                    ref error,
                    phase: ErrorPhase::RequestParsing
                } if error == "empty method"
            ),
            "Expected ParseFailed for invalid method, got {:?}",
            planned
        );
    }

    #[test]
    fn plan_request_audit_with_invalid_uri() {
        let method: Result<HttpMethod, String> =
            Ok(HttpMethod::try_new("GET".to_string()).unwrap());
        let uri: Result<RequestUri, String> = Err("empty uri".to_string());
        let headers = vec![];

        let planned = plan_request_audit(method, uri, headers, BodySize::from(0));

        assert!(
            matches!(
                planned,
                PlannedRequestAudit::ParseFailed {
                    ref error,
                    phase: ErrorPhase::RequestParsing
                } if error == "empty uri"
            ),
            "Expected ParseFailed for invalid uri, got {:?}",
            planned
        );
    }

    #[test]
    fn plan_response_audit_with_valid_status() {
        let status: Result<HttpStatusCode, u16> = Ok(HttpStatusCode::try_new(200).unwrap());
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let duration = DurationMillis::from(42);

        let planned = plan_response_audit(status, headers.clone(), BodySize::from(128), duration);

        assert!(
            matches!(
                planned,
                PlannedResponseAudit::Received {
                    status: _,
                    headers: _,
                    body_size,
                    duration_ms
                } if *body_size.as_ref() == 128 && *duration_ms.as_ref() == 42
            ),
            "Expected Received variant, got {:?}",
            planned
        );
    }

    #[test]
    fn plan_response_audit_with_invalid_status() {
        let status: Result<HttpStatusCode, u16> = Err(999);
        let headers = vec![];

        let planned =
            plan_response_audit(status, headers, BodySize::from(0), DurationMillis::from(0));

        assert!(
            matches!(
                planned,
                PlannedResponseAudit::ParseFailed {
                    ref error,
                    phase: ErrorPhase::ResponseReceiving
                } if error.contains("999")
            ),
            "Expected ParseFailed for invalid status, got {:?}",
            planned
        );
    }
}
