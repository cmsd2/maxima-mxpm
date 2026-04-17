//! Core types and utilities for the Maxima package ecosystem.
//!
//! This crate provides foundational types shared across mxpm tools:
//! manifest parsing, Maxima directory resolution, and install metadata.
//! It has minimal dependencies (serde, toml, dirs) so downstream
//! consumers like the LSP and MCP servers can use it without pulling
//! in the full mxpm CLI dependency tree.

pub mod manifest;
pub mod paths;
