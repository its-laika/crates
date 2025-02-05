//! # Laika's namespace crate
//!
//! This module contains multiple submodules (included via feature flags) with
//! different functionalities. They're all grouped under the `laika` namespace,
//! providing some kind of scoped crates (avoiding naming conflicts).
//!
//! ## Submodules / Features
//!
//! ### [`shotgun`]
//!
//! Shotgun is a simple one-shot single producer, multiple consumer (SPMC)
//! channel. It internally uses `std::sync::Mutex` and `std::sync::Arc` and does
//! not contain any unsafe code.  
//! See module documentation for more information.
#[cfg(feature = "shotgun")]
pub mod shotgun;
