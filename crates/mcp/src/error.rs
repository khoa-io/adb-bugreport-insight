//! `ToolError`, and the `ArchiveError` → wire mapping.
//!
//! Deliberately **free of `rmcp::ErrorData`**. The classification this module
//! performs — transport vs domain — is what [`crate::adapter`] turns into a
//! JSON-RPC error or a caller-visible tool error, and it has to be testable
//! without a protocol stack.

use bugreport_insight_core::{ArchiveError, ArchiveStatus, EntryId};
use serde::Serialize;
use serde_json::{Value, json};

/// Which failure mode the caller sees, per `design/05-mcp-server-design.md` §4.
///
/// The choice is by *whose problem it is*, not by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorKind {
    /// The server cannot route or run the request at all: malformed or
    /// contradictory parameters. Becomes a JSON-RPC error, which clients render
    /// opaquely. Carries no agent-recoverable state.
    Protocol,
    /// The request was well-formed and routed, but this one target could not be
    /// satisfied. Becomes `CallToolResult::error` with a structured body, so the
    /// agent sees the message and can retry with a different `entry_id`.
    Domain,
}

#[derive(Debug, Clone)]
pub struct ToolError {
    pub kind: ToolErrorKind,
    /// Stable machine code. `message` is prose; **this** is the contract.
    pub code: String,
    pub message: String,
    pub data: Option<Value>,
}

impl ToolError {
    pub fn protocol(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ToolErrorKind::Protocol,
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn domain(code: impl Into<String>, message: impl Into<String>, data: Value) -> Self {
        Self {
            kind: ToolErrorKind::Domain,
            code: code.into(),
            message: message.into(),
            data: Some(data),
        }
    }

    /// `ArchiveError` → [`ToolError`].
    ///
    /// The class split is taken from the design table and is exhaustive over
    /// `ArchiveError` by way of `is_per_entry` plus the three transport-class
    /// variants. The `_` arm is unreachable today; `ArchiveError` is
    /// `#[non_exhaustive]`, so an unmatched variant must degrade to a protocol
    /// error rather than panic.
    pub fn from_archive(e: &ArchiveError) -> Self {
        if e.is_per_entry() {
            return Self::domain(e.code(), e.to_string(), archive_error_data(e));
        }
        match e {
            ArchiveError::InvalidGlob { .. }
            | ArchiveError::ConflictingRangeParams
            | ArchiveError::PatternModeUnavailable { .. } => {
                Self::protocol(e.code(), e.to_string())
            }
            _ => Self::domain(e.code(), e.to_string(), archive_error_data(e)),
        }
    }

    /// The structured body an agent receives on a domain failure.
    pub fn to_json(&self) -> Value {
        json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "data": self.data,
            }
        })
    }
}

/// The minimum an agent needs to act: the code, and the ids it can retry with.
fn archive_error_data(e: &ArchiveError) -> Value {
    let mut data = serde_json::Map::new();
    if let Some(id) = entry_id_of(e) {
        data.insert("entry_id".into(), json!(id.0));
    }
    if e.is_per_entry() {
        data.insert("status".into(), json!(ArchiveStatus::from_error(e).code()));
    }
    Value::Object(data)
}

fn entry_id_of(e: &ArchiveError) -> Option<EntryId> {
    match e {
        ArchiveError::EntryNotFound(id)
        | ArchiveError::EntryIsDirectory(id)
        | ArchiveError::EntryEncrypted { id, .. }
        | ArchiveError::UnsupportedCompression { id, .. }
        | ArchiveError::UnreadableEntry { id, .. }
        | ArchiveError::ByteRangeOutOfBounds { id, .. }
        | ArchiveError::LineRangeOutOfBounds { id, .. } => Some(*id),
        _ => None,
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ToolError {}

/// Serialises as the body an agent reads. Used by the adapter and by tests.
#[derive(Debug, Serialize)]
pub struct ToolErrorBody<'a> {
    pub code: &'a str,
    pub message: &'a str,
    pub data: Option<&'a Value>,
}
