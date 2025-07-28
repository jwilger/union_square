//! Buffer management for audit events
//!
//! This module provides functionality to buffer request/response bodies
//! from chunk events and process them when complete.

use crate::proxy::types::{ChunkOffset, RequestId};
use std::collections::HashMap;

/// Buffered data for a request or response
#[derive(Debug, Clone)]
pub struct BufferedData {
    chunks: Vec<(ChunkOffset, Vec<u8>)>,
    total_size: usize,
}

impl BufferedData {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            total_size: 0,
        }
    }

    /// Add a chunk to the buffer
    pub fn add_chunk(&mut self, offset: ChunkOffset, data: Vec<u8>) {
        self.total_size = (*offset.as_ref() + data.len()).max(self.total_size);
        self.chunks.push((offset, data));
    }

    /// Set the expected total size (for cases where we know it upfront)
    pub fn set_total_size(&mut self, size: usize) {
        self.total_size = size;
    }

    /// Check if the buffer is complete (has all data)
    pub fn is_complete(&self) -> bool {
        if self.chunks.is_empty() {
            return false;
        }

        // If we don't have a total_size set, we can't determine completeness
        if self.total_size == 0 {
            return false;
        }

        // Sort chunks by offset
        let mut sorted_chunks = self.chunks.clone();
        sorted_chunks.sort_by_key(|(offset, _)| *offset.as_ref());

        // Check for gaps
        let mut current_pos = 0;
        for (offset, data) in &sorted_chunks {
            if *offset.as_ref() != current_pos {
                return false; // Gap found
            }
            current_pos += data.len();
        }

        current_pos == self.total_size
    }

    /// Reconstruct the complete data from chunks
    pub fn reconstruct(&self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }

        let mut sorted_chunks = self.chunks.clone();
        sorted_chunks.sort_by_key(|(offset, _)| *offset.as_ref());

        let mut result = Vec::with_capacity(self.total_size);
        for (_, data) in sorted_chunks {
            result.extend_from_slice(&data);
        }

        Some(result)
    }
}

/// Manager for buffering audit data across multiple requests
pub struct AuditBufferManager {
    request_buffers: HashMap<RequestId, BufferedData>,
    response_buffers: HashMap<RequestId, BufferedData>,
}

impl AuditBufferManager {
    pub fn new() -> Self {
        Self {
            request_buffers: HashMap::new(),
            response_buffers: HashMap::new(),
        }
    }

    /// Add a request chunk
    pub fn add_request_chunk(&mut self, request_id: RequestId, offset: ChunkOffset, data: Vec<u8>) {
        self.request_buffers
            .entry(request_id)
            .or_insert_with(BufferedData::new)
            .add_chunk(offset, data);
    }

    /// Add a response chunk
    pub fn add_response_chunk(
        &mut self,
        request_id: RequestId,
        offset: ChunkOffset,
        data: Vec<u8>,
    ) {
        self.response_buffers
            .entry(request_id)
            .or_insert_with(BufferedData::new)
            .add_chunk(offset, data);
    }

    /// Check if request body is complete and return it
    pub fn get_complete_request_body(&mut self, request_id: &RequestId) -> Option<Vec<u8>> {
        if let Some(buffer) = self.request_buffers.get(request_id) {
            if let Some(data) = buffer.reconstruct() {
                // Remove the buffer once we've reconstructed it
                self.request_buffers.remove(request_id);
                return Some(data);
            }
        }
        None
    }

    /// Check if response body is complete and return it
    pub fn get_complete_response_body(&mut self, request_id: &RequestId) -> Option<Vec<u8>> {
        if let Some(buffer) = self.response_buffers.get(request_id) {
            if let Some(data) = buffer.reconstruct() {
                // Remove the buffer once we've reconstructed it
                self.response_buffers.remove(request_id);
                return Some(data);
            }
        }
        None
    }

    /// Clean up old buffers for a request
    pub fn cleanup_request(&mut self, request_id: &RequestId) {
        self.request_buffers.remove(request_id);
        self.response_buffers.remove(request_id);
    }
}

impl Default for AuditBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffered_data_complete() {
        let mut buffer = BufferedData::new();

        // Add chunks in order
        buffer.add_chunk(ChunkOffset::from(0), vec![1, 2, 3]);
        buffer.add_chunk(ChunkOffset::from(3), vec![4, 5, 6]);

        assert!(buffer.is_complete());
        assert_eq!(buffer.reconstruct(), Some(vec![1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn test_buffered_data_with_gap() {
        let mut buffer = BufferedData::new();

        // Add chunks with a gap
        buffer.add_chunk(ChunkOffset::from(0), vec![1, 2, 3]);
        buffer.add_chunk(ChunkOffset::from(6), vec![7, 8, 9]); // Gap at 3-5

        assert!(!buffer.is_complete());
        assert_eq!(buffer.reconstruct(), None);
    }

    #[test]
    fn test_buffered_data_out_of_order() {
        let mut buffer = BufferedData::new();

        // Add chunks out of order
        buffer.add_chunk(ChunkOffset::from(3), vec![4, 5, 6]);
        buffer.add_chunk(ChunkOffset::from(0), vec![1, 2, 3]);

        assert!(buffer.is_complete());
        assert_eq!(buffer.reconstruct(), Some(vec![1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn test_audit_buffer_manager() {
        let mut manager = AuditBufferManager::new();
        let request_id = RequestId::new();

        // For this test, we need a different approach since we need to know total size
        // Let's test the cleanup functionality instead
        manager.add_request_chunk(request_id, ChunkOffset::from(0), vec![1, 2, 3]);
        manager.add_response_chunk(request_id, ChunkOffset::from(0), vec![4, 5, 6]);

        // Cleanup should remove both buffers
        manager.cleanup_request(&request_id);

        // After cleanup, trying to get data returns None
        assert!(manager.get_complete_request_body(&request_id).is_none());
        assert!(manager.get_complete_response_body(&request_id).is_none());
    }
}
