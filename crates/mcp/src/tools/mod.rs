//! The tool layer. Protocol-free by construction.
//!
//! Every module below this one may be exercised with a plain `Archive` and no
//! server, no transport, and no `rmcp` type in scope.

pub mod envelope;
pub mod handlers;
pub mod params;

mod pagination;
