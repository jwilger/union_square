//! Integration tests for provider-based routing

#[cfg(test)]
mod tests {
    use crate::proxy::types::ApiKey;
    use crate::proxy::{AuthConfig, ProxyConfig, ProxyService};
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use mockito::{Matcher, Server};
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_bedrock_provider_integration() {
        let mut mock_server = Server::new_async().await;
        let mock_url = mock_server.url();

        // Override the bedrock region to use mock URL
        std::env::set_var("BEDROCK_ENDPOINT_OVERRIDE", mock_url.clone());

        // Create mock Bedrock endpoint
        let mock = mock_server
            .mock("POST", "/model/anthropic.claude-3-sonnet-20240229/invoke")
            .match_header(
                "authorization",
                Matcher::Regex(r"AWS4-HMAC-SHA256.*".to_string()),
            )
            .match_header("x-amz-date", Matcher::Any)
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "msg_123",
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "text",
                        "text": "Hello from Claude via Union Square!"
                    }],
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 8
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Create proxy with Bedrock provider enabled
        let config = ProxyConfig {
            bedrock_region: Some("us-east-1".to_string()),
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let proxy_service = ProxyService::new(config);

        // Create auth config
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        // Create router
        let router = proxy_service.into_router(auth_config);

        // Start proxy server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind listener");
        let proxy_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("Server failed");
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create client and test request
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let request = Request::builder()
            .method("POST")
            .uri(format!(
                "http://{proxy_addr}/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke"
            ))
            .header("x-api-key", "test-key")
            .header(
                "authorization",
                "AWS4-HMAC-SHA256 Credential=test/20250126/us-east-1/bedrock/aws4_request",
            )
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "anthropic_version": "bedrock-2023-05-31",
                    "messages": [{
                        "role": "user",
                        "content": "Hello!"
                    }],
                    "max_tokens": 100
                })
                .to_string(),
            ))
            .unwrap();

        let response = client.request(request).await.unwrap();

        // Verify response
        let status = response.status();
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

        if status != StatusCode::OK {
            let body_str = String::from_utf8_lossy(&body_bytes);
            panic!("Response status: {status}, body: {body_str}");
        }

        let response_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            response_json["content"][0]["text"],
            "Hello from Claude via Union Square!"
        );
        assert_eq!(response_json["usage"]["input_tokens"], 10);

        // Verify mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fallback_to_header_routing() {
        let mut mock_server = Server::new_async().await;

        // Create mock target server
        let mock = mock_server
            .mock("GET", "/api/test")
            .with_status(200)
            .with_body("Header routing works")
            .create_async()
            .await;

        // Create proxy
        let config = ProxyConfig::default();
        let proxy_service = ProxyService::new(config);

        // Create auth config
        let mut auth_config = AuthConfig::default();
        auth_config
            .api_keys
            .insert(ApiKey::try_new("test-key".to_string()).unwrap());

        // Create router
        let router = proxy_service.into_router(auth_config);

        // Start proxy server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind listener");
        let proxy_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("Server failed");
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create client and test request
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let request = Request::builder()
            .method("GET")
            .uri(format!("http://{proxy_addr}/api/test"))
            .header("x-api-key", "test-key")
            .header("x-target-url", mock_server.url())
            .body(Body::empty())
            .unwrap();

        let response = client.request(request).await.unwrap();

        // Verify response
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body_bytes, "Header routing works");

        // Verify mock was called
        mock.assert_async().await;
    }
}
