//! The envelope for successful tool results, including partial results.
//!
//! `REQ-MCP-06` is enforced *structurally*: the four bounding fields are not
//! optional and not `skip_serializing_if`, so they are present on every single
//! successful result whether or not the tool was capped. An agent never has to wonder
//! whether a missing `truncated` means "not truncated" or "the tool forgot".
//! Domain and protocol failures use separate error shapes without these fields.

use bugreport_insight_core::{ArchiveStatus, StoppedReason};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ToolEnvelope {
    /// `REQ-MCP-08`. Never inferred from "no results" — it is decided by the
    /// tool, and a failure never produces `Empty`.
    pub status: ArchiveStatus,
    /// Tool-specific payload. Shape is documented per tool in `docs/`.
    pub data: Value,
    pub truncated: bool,
    pub stopped_reason: Option<StoppedReason>,
    pub omitted_count: u64,
    pub notes: Vec<String>,
}

impl ToolEnvelope {
    pub fn ok(data: Value) -> Self {
        Self {
            status: ArchiveStatus::Ok,
            data,
            truncated: false,
            stopped_reason: None,
            omitted_count: 0,
            notes: Vec::new(),
        }
    }

    /// The tool ran to completion and there was nothing to report — the one and
    /// only constructor of `Empty`.
    pub fn empty(data: Value, note: impl Into<String>) -> Self {
        Self {
            status: ArchiveStatus::Empty,
            data,
            truncated: false,
            stopped_reason: None,
            omitted_count: 0,
            notes: vec![note.into()],
        }
    }

    pub fn with_cap(mut self, reason: StoppedReason, omitted: u64) -> Self {
        self.truncated = true;
        self.stopped_reason = Some(reason);
        self.omitted_count = omitted;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
