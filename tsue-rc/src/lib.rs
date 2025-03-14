//! # Vice Server Library
//!
//! vice is a server library where it provide project architecture from the start
//!
//! it sacrifice customability for it
// impl Future vs type Future vs generic Future
// - impl Future: can be async fn, type cannot be referenced externally, no double implementation
// - type Future: no async fn, type can be referenced externally, no double implementation
// - generic Future: no async fn, type ? be referenced externally, can double implementation
//
// impl Future
// - can be async fn
// - can contains unnamed future without boxing, like async fn or private future type
// - future type cannot be referenced externally
// - cannot have double implementation
//
// generic Future
// - cannot be async fn
// - cannot contains unnamed future without boxing, like async fn or private future type
// - future type cannot be referenced externally
// - can have double implementation
//
// type Future
// - cannot be async fn
// - cannot contains unnamed future without boxing, like async fn or private future type (unstable)
// - future type can be referenced externally
// - cannot have double implementation
pub mod bytestr;
pub mod http;
pub mod request;
pub mod response;

pub mod task;
pub mod body;

pub mod service;
pub mod runtime;

