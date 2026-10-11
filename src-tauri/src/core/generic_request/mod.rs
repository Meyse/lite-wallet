//! Requests from websites and apps (`GenericRequest` envelopes).
//!
//! TypeScript parses each detail with verus-typescript-primitives for review.
//! This module re-reads the verified request bytes and derives every value the
//! wallet acts on, so nothing that is signed, paid, shared or broadcast comes
//! from the UI (ADR 0002).

pub(crate) mod acceptance;
pub(crate) mod credentials;
pub(crate) mod details;
pub(crate) mod legacy;
pub(crate) mod names;
pub(crate) mod private_crypto;
pub(crate) mod request;
pub(crate) mod response;
pub mod store;
pub(crate) mod wire;
