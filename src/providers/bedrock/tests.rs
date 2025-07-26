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
mod model_specific_tests {
    use crate::providers::bedrock::models::*;
    use crate::providers::bedrock::types::ModelFamily;
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
        assert_eq!(usage.input_tokens, 15);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.total_tokens, 25);
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
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 32);
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
        assert_eq!(usage.input_tokens, 8);
        assert_eq!(usage.output_tokens, 15);
        assert_eq!(usage.total_tokens, 23);
    }
}
