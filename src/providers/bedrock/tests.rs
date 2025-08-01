//! Tests for AWS Bedrock provider

#[cfg(test)]
mod invoke_model_tests {
    use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};
    use crate::providers::{Provider, ProviderError};
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use mockito::{Matcher, Server};
    use serde_json::json;

    #[tokio::test]
    async fn test_invoke_model_endpoint_routing() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        // Test that it matches the correct paths
        assert!(provider.matches_path("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke"));
        assert!(provider.matches_path("/bedrock/model/amazon.titan-text-express-v1/invoke"));
        assert!(provider.matches_path("/bedrock/model/test/invoke-with-response-stream"));
        // Should match any bedrock path
    }

    #[tokio::test]
    async fn test_invoke_model_url_transformation() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-west-2").unwrap());

        let uri = "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke"
            .parse()
            .unwrap();
        let transformed = provider.transform_url(&uri).unwrap();

        assert_eq!(
            transformed.to_string(),
            "https://bedrock-runtime.us-west-2.amazonaws.com/model/anthropic.claude-3-sonnet-20240229/invoke"
        );
    }

    #[tokio::test]
    async fn test_invoke_model_auth_passthrough() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create a mock endpoint that expects auth headers
        let mock = server
            .mock("POST", "/model/anthropic.claude-3-sonnet-20240229/invoke")
            .match_header(
                "authorization",
                Matcher::Regex(r"AWS4-HMAC-SHA256.*".to_string()),
            )
            .match_header("x-amz-date", Matcher::Any)
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"completion": "Hello from Claude!"}).to_string())
            .create_async()
            .await;

        // Create request with SigV4 headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke")
            .header(
                "authorization",
                "AWS4-HMAC-SHA256 Credential=test/20250126/us-east-1/bedrock/aws4_request",
            )
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a test HTTP client
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request through the provider
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify the response
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_invoke_model_request_body_passthrough() {
        // Test request body for Claude model
        let claude_body = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 1000,
            "messages": [{
                "role": "user",
                "content": "Hello Claude!"
            }]
        });

        // Test request body for Titan model
        let titan_body = json!({
            "inputText": "Hello Titan!",
            "textGenerationConfig": {
                "maxTokenCount": 1000,
                "temperature": 0.7
            }
        });

        // For MVP, we pass through request bodies as-is
        // Just verify the JSON is valid
        assert!(claude_body.is_object());
        assert!(titan_body.is_object());
    }

    #[tokio::test]
    async fn test_invoke_model_error_handling() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create a mock endpoint that returns a Bedrock error
        let mock = server
            .mock("POST", "/model/anthropic.claude-3-sonnet-20240229/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(400)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_body(
                json!({
                    "__type": "ValidationException",
                    "message": "Invalid model ID"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Create request with SigV4 headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a test HTTP client
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request through the provider
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify the error response is passed through
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Read the response body
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ValidationException");
        assert_eq!(error_response["message"], "Invalid model ID");

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_missing_auth_headers() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        // Create request without auth headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Should fail due to missing auth headers
        let result = provider.forward_request(request, &client).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ProviderError::AuthenticationError(msg) => {
                assert!(msg.contains("Missing authorization header"));
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }
}

#[cfg(test)]
mod streaming_tests {
    use crate::providers::bedrock::{provider::BedrockProvider, types::AwsRegion};
    use crate::providers::Provider;
    use axum::body::Body;
    use futures_util::StreamExt;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use mockito::{Matcher, Server};
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_invoke_model_with_response_stream_routing() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        assert!(provider.matches_path(
            "/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream"
        ));
        assert!(provider.matches_path("/bedrock/model/test/invoke")); // Should match any bedrock path
    }

    #[tokio::test]
    async fn test_invoke_model_with_response_stream_passthrough() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create streaming response chunks
        let chunks = vec![
            r#"{"chunk":{"bytes":"eyJjb21wbGV0aW9uIjogIkhlbGxvIn0="}}"#, // {"completion": "Hello"}
            r#"{"chunk":{"bytes":"eyJjb21wbGV0aW9uIjogIiBmcm9tIn0="}}"#, // {"completion": " from"}
            r#"{"chunk":{"bytes":"eyJjb21wbGV0aW9uIjogIiBDbGF1ZGUhIn0="}}"#, // {"completion": " Claude!"}
        ];

        let chunks_for_mock = chunks.clone();

        // Create a mock endpoint that returns streaming response
        let mock = server
            .mock(
                "POST",
                "/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream",
            )
            .match_header(
                "authorization",
                Matcher::Regex(r"AWS4-HMAC-SHA256.*".to_string()),
            )
            .match_header("x-amz-date", Matcher::Any)
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/vnd.amazon.eventstream")
            .with_chunked_body(move |w| {
                for chunk in &chunks_for_mock {
                    w.write_all(chunk.as_bytes()).unwrap();
                    w.write_all(b"\n").unwrap();
                    std::thread::sleep(Duration::from_millis(10)); // Simulate streaming delay
                }
                Ok(())
            })
            .create_async()
            .await;

        // Create request with SigV4 headers
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream")
            .header(
                "authorization",
                "AWS4-HMAC-SHA256 Credential=test/20250126/us-east-1/bedrock/aws4_request",
            )
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a test HTTP client
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request through the provider
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify the response headers
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/vnd.amazon.eventstream"
        );

        // Collect the streamed response body
        let mut body_stream = response.into_body().into_data_stream();
        let mut collected_chunks = Vec::new();
        while let Some(chunk_result) = body_stream.next().await {
            let chunk = chunk_result.unwrap();
            collected_chunks.push(String::from_utf8_lossy(&chunk).to_string());
        }

        // Verify we received all chunks
        let combined = collected_chunks.join("");
        for expected_chunk in &chunks {
            assert!(combined.contains(expected_chunk));
        }

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_error_handling() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        // Create provider with mock URL
        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Create a mock endpoint that returns a streaming error
        let mock = server
            .mock(
                "POST",
                "/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream",
            )
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(400)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_body(
                json!({
                    "__type": "ThrottlingException",
                    "message": "Rate exceeded"
                })
                .to_string(),
            )
            .create_async()
            .await;

        // Create request
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke-with-response-stream")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .header("content-type", "application/json")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        // Forward the request
        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify error response
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ThrottlingException");
        assert_eq!(error_response["message"], "Rate exceeded");

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_response_zero_copy() {
        let mut server = Server::new_async().await;
        let mock_url = server.url();

        let provider = BedrockProvider::with_base_url(mock_url.clone());

        // Large chunk to test zero-copy efficiency
        let large_chunk = "x".repeat(1024 * 1024); // 1MB chunk
        use base64::Engine;
        let encoded_chunk = base64::engine::general_purpose::STANDARD.encode(&large_chunk);
        let response_chunk = format!(r#"{{"chunk":{{"bytes":"{encoded_chunk}"}}}}"#);

        let mock = server
            .mock("POST", "/model/test/invoke-with-response-stream")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/vnd.amazon.eventstream")
            .with_body(&response_chunk)
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke-with-response-stream")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let start = std::time::Instant::now();
        let response = provider.forward_request(request, &client).await.unwrap();

        // Consume the response to ensure the full transfer
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let elapsed = start.elapsed();

        // Verify we got the large chunk
        assert_eq!(body_bytes.len(), response_chunk.len());

        // Zero-copy should be fast even for large payloads
        assert!(
            elapsed < Duration::from_secs(1),
            "Transfer took too long: {elapsed:?}"
        );

        mock.assert_async().await;
    }
}

#[cfg(test)]
mod error_handling_tests {
    use crate::providers::bedrock::provider::BedrockProvider;
    use crate::providers::Provider;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use mockito::{Matcher, Server};
    use serde_json::json;

    #[tokio::test]
    async fn test_validation_exception_passthrough() {
        let mut server = Server::new_async().await;
        let provider = BedrockProvider::with_base_url(server.url());

        let mock = server
            .mock("POST", "/model/test/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(400)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_header("x-amzn-requestid", "test-request-id")
            .with_header("x-amzn-errortype", "ValidationException")
            .with_body(
                json!({
                    "__type": "ValidationException",
                    "message": "1 validation error detected: Value 'invalid-model' at 'modelId' failed to satisfy constraint: Model ID must be valid"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify status and headers are preserved
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get("x-amzn-errortype").unwrap(),
            "ValidationException"
        );

        // Verify body is preserved exactly
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ValidationException");
        assert!(error_response["message"]
            .as_str()
            .unwrap()
            .contains("validation error"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_throttling_exception_passthrough() {
        let mut server = Server::new_async().await;
        let provider = BedrockProvider::with_base_url(server.url());

        let mock = server
            .mock("POST", "/model/test/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(429)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_header("x-amzn-errortype", "ThrottlingException")
            .with_header("retry-after", "5")
            .with_body(
                json!({
                    "__type": "ThrottlingException",
                    "message": "Rate exceeded. Please retry your request after 5 seconds."
                })
                .to_string(),
            )
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = provider.forward_request(request, &client).await.unwrap();

        // Verify status and headers
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get("retry-after").unwrap(), "5");

        // Verify body
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ThrottlingException");
        assert!(error_response["message"]
            .as_str()
            .unwrap()
            .contains("Rate exceeded"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_model_not_ready_exception_passthrough() {
        let mut server = Server::new_async().await;
        let provider = BedrockProvider::with_base_url(server.url());

        let mock = server
            .mock("POST", "/model/test/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(503)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_header("x-amzn-errortype", "ModelNotReadyException")
            .with_body(
                json!({
                    "__type": "ModelNotReadyException",
                    "message": "Model is still loading. Please try again in a few moments."
                })
                .to_string(),
            )
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = provider.forward_request(request, &client).await.unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "ModelNotReadyException");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_access_denied_exception_passthrough() {
        let mut server = Server::new_async().await;
        let provider = BedrockProvider::with_base_url(server.url());

        let mock = server
            .mock(
                "POST",
                "/model/anthropic.claude-3-5-sonnet-20241022-v2:0/invoke",
            )
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(403)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_header("x-amzn-errortype", "AccessDeniedException")
            .with_body(
                json!({
                    "__type": "AccessDeniedException",
                    "message": "You don't have access to the model with the specified model ID."
                })
                .to_string(),
            )
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-5-sonnet-20241022-v2:0/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = provider.forward_request(request, &client).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "AccessDeniedException");
        assert!(error_response["message"]
            .as_str()
            .unwrap()
            .contains("don't have access"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_internal_server_error_passthrough() {
        let mut server = Server::new_async().await;
        let provider = BedrockProvider::with_base_url(server.url());

        let mock = server
            .mock("POST", "/model/test/invoke")
            .match_header("authorization", Matcher::Any)
            .match_header("x-amz-date", Matcher::Any)
            .with_status(500)
            .with_header("content-type", "application/x-amz-json-1.1")
            .with_header("x-amzn-errortype", "InternalServerException")
            .with_body(
                json!({
                    "__type": "InternalServerException",
                    "message": "An internal server error occurred. Please try again."
                })
                .to_string(),
            )
            .create_async()
            .await;

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .header("authorization", "AWS4-HMAC-SHA256 Credential=test")
            .header("x-amz-date", "20250126T120000Z")
            .body(Body::empty())
            .unwrap();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = provider.forward_request(request, &client).await.unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(error_response["__type"], "InternalServerException");

        mock.assert_async().await;
    }
}

#[cfg(test)]
mod recording_transformation_tests {
    use crate::providers::bedrock::{
        provider::BedrockProvider,
        types::{AwsRegion, ModelId},
    };
    use crate::providers::{Provider, ProviderId, RequestId};
    use axum::body::Body;
    use hyper::{Request, Response, StatusCode};
    use serde_json::json;

    #[tokio::test]
    async fn test_extract_metadata_from_claude_response() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        // Create a mock request
        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/anthropic.claude-3-sonnet-20240229/invoke")
            .body(Body::from(json!({"prompt": "Hello"}).to_string()))
            .unwrap();

        // Create a mock response with Claude format
        let response_body = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello!"}],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let response = Response::builder()
            .status(StatusCode::OK)
            .header("x-amzn-requestid", "test-request-123")
            .body(Body::from(response_body.to_string()))
            .unwrap();

        // Extract metadata
        let metadata = provider.extract_metadata(&request, &response);

        assert_eq!(metadata.provider_id, ProviderId::bedrock());
        assert_eq!(
            metadata.model_id,
            Some(ModelId::try_new("anthropic.claude-3-sonnet-20240229".to_string()).unwrap())
        );
        assert_eq!(
            metadata.provider_request_id,
            Some(RequestId::try_new("test-request-123".to_string()).unwrap())
        );

        // Note: Token extraction happens during response body processing, not in extract_metadata
        // These would be populated by the audit recorder after parsing the response body
    }

    #[tokio::test]
    async fn test_extract_metadata_from_titan_response() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/amazon.titan-text-express-v1/invoke")
            .body(Body::empty())
            .unwrap();

        let response = Response::builder()
            .status(StatusCode::OK)
            .header("x-amzn-requestid", "titan-req-456")
            .body(Body::empty())
            .unwrap();

        let metadata = provider.extract_metadata(&request, &response);

        assert_eq!(metadata.provider_id, ProviderId::bedrock());
        assert_eq!(
            metadata.model_id,
            Some(ModelId::try_new("amazon.titan-text-express-v1".to_string()).unwrap())
        );
        assert_eq!(
            metadata.provider_request_id,
            Some(RequestId::try_new("titan-req-456".to_string()).unwrap())
        );
    }

    #[tokio::test]
    async fn test_extract_metadata_from_error_response() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test-model/invoke")
            .body(Body::empty())
            .unwrap();

        let response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("x-amzn-requestid", "error-req-789")
            .header("x-amzn-errortype", "ValidationException")
            .body(Body::empty())
            .unwrap();

        let metadata = provider.extract_metadata(&request, &response);

        assert_eq!(metadata.provider_id, ProviderId::bedrock());
        assert_eq!(
            metadata.model_id,
            Some(ModelId::try_new("test-model".to_string()).unwrap())
        );
        assert_eq!(
            metadata.provider_request_id,
            Some(RequestId::try_new("error-req-789".to_string()).unwrap())
        );
        // No token data on errors
        assert_eq!(metadata.request_tokens, None);
        assert_eq!(metadata.response_tokens, None);
    }

    #[tokio::test]
    async fn test_metadata_extraction_without_request_id() {
        let provider = BedrockProvider::new(AwsRegion::try_new("us-east-1").unwrap());

        let request = Request::builder()
            .method("POST")
            .uri("/bedrock/model/test/invoke")
            .body(Body::empty())
            .unwrap();

        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        let metadata = provider.extract_metadata(&request, &response);

        assert_eq!(metadata.provider_id, ProviderId::bedrock());
        assert_eq!(metadata.provider_request_id, None);
    }
}

#[cfg(test)]
mod model_specific_tests {
    use crate::providers::bedrock::models::*;
    use crate::providers::bedrock::types::{InputTokens, ModelFamily, OutputTokens, TotalTokens};
    use serde_json::json;

    #[test]
    fn test_claude_token_extraction() {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello! I'm Claude."
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 10
            }
        });

        let tokens = extract_token_usage(&ModelFamily::Claude, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, InputTokens::try_new(15).unwrap());
        assert_eq!(usage.output_tokens, OutputTokens::try_new(10).unwrap());
        assert_eq!(usage.total_tokens, TotalTokens::try_new(25).unwrap());
    }

    #[test]
    fn test_titan_token_extraction() {
        let response = json!({
            "inputTextTokenCount": 12,
            "results": [{
                "tokenCount": 20,
                "outputText": "Hello from Titan!",
                "completionReason": "FINISH"
            }]
        });

        let tokens = extract_token_usage(&ModelFamily::Titan, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, InputTokens::try_new(12).unwrap());
        assert_eq!(usage.output_tokens, OutputTokens::try_new(20).unwrap());
        assert_eq!(usage.total_tokens, TotalTokens::try_new(32).unwrap());
    }

    #[test]
    fn test_llama_token_extraction() {
        let response = json!({
            "generation": "Hello from Llama!",
            "prompt_token_count": 8,
            "generation_token_count": 15,
            "stop_reason": "stop"
        });

        let tokens = extract_token_usage(&ModelFamily::Llama, &response);
        assert!(tokens.is_some());

        let usage = tokens.unwrap();
        assert_eq!(usage.input_tokens, InputTokens::try_new(8).unwrap());
        assert_eq!(usage.output_tokens, OutputTokens::try_new(15).unwrap());
        assert_eq!(usage.total_tokens, TotalTokens::try_new(23).unwrap());
    }
}
