// src/models/mod.rs
//
// OpenAI API compatible data models
//
// This module provides Rust structs that match the exact OpenAI API specification
// for chat completions, streaming, models endpoints, and error handling.
// All models maintain 100% compatibility with OpenAI API format.

pub mod request;
pub mod response;
pub mod error;
pub mod common;

pub use request::*;
pub use response::*;
pub use error::*;
pub use common::*;