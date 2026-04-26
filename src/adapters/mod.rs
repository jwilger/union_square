//! Adapter modules for converting boundary DTOs to domain types
//!
//! Adapters sit at the boundary between the imperative shell (proxy, HTTP, etc.)
//! and the functional core (domain). They parse structural data into semantic
//! domain facts and handle conversion errors explicitly.

pub mod llm_request_parser;
pub mod proxy_audit;
