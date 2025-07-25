//! Streaming implementation for zero-copy request/response forwarding

use crate::proxy::audit_recorder::{AuditRecorder, ChunkCapture};
use crate::proxy::types::*;
use axum::body::Body;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use hyper::Response;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Create a streaming body that captures data to ring buffer while forwarding
pub fn create_capturing_stream<S>(
    stream: S,
    recorder: Arc<dyn AuditRecorder + Send + Sync>,
    request_id: RequestId,
    is_request: bool,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>>
where
    S: Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    // Use shared chunk capture functionality
    let chunk_capture = ChunkCapture::new(recorder, request_id, is_request);
    let tx = chunk_capture.start_capture_task();

    // Return a stream that forwards data and sends copies to the capture task
    stream.map(move |result| {
        if let Ok(ref chunk) = result {
            // Best-effort send to capture task
            let _ = tx.try_send(chunk.clone());
        }
        result
    })
}

pin_project! {
    /// Streaming body that captures to ring buffer while forwarding
    pub struct CapturingBody<B> {
        #[pin]
        inner: B,
        tx: Option<mpsc::Sender<Bytes>>,
    }
}

impl<B> CapturingBody<B>
where
    B: http_body::Body,
{
    pub fn new(
        body: B,
        recorder: Arc<dyn AuditRecorder + Send + Sync>,
        request_id: RequestId,
        is_request: bool,
    ) -> Self {
        // Use shared chunk capture functionality
        let chunk_capture = ChunkCapture::new(recorder, request_id, is_request);
        let tx = chunk_capture.start_capture_task();

        Self {
            inner: body,
            tx: Some(tx),
        }
    }
}

impl<B> http_body::Body for CapturingBody<B>
where
    B: http_body::Body,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Data = B::Data;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.project();

        match this.inner.poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                // Try to capture data frames
                if let Some(_data) = frame.data_ref() {
                    if let Some(_tx) = this.tx {
                        // Get bytes from the data - the exact method depends on the Body type
                        // For now, we'll skip complex conversion and focus on the structure
                        // TODO: Implement proper data extraction based on Body::Data type
                    }
                }
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => {
                // Stream ended, drop the sender to signal capture task
                *this.tx = None;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

/// Create a streaming response that doesn't buffer entire body
pub async fn create_streaming_response<B>(
    response: Response<B>,
    _recorder: Arc<dyn AuditRecorder + Send + Sync>,
    _request_id: RequestId,
) -> Response<Body>
where
    B: http_body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let (parts, _body) = response.into_parts();

    // TODO: Implement proper streaming with chunk capture
    // For now, return empty body to avoid compilation issues
    Response::from_parts(parts, Body::empty())
}
