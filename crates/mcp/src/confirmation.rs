//! Bounded, request-bound approval before archive content IO.
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bugreport_insight_core::{Archive, EntryId, caps};
use serde_json::{Value, json};

use crate::error::ToolError;

const CAPACITY: usize = 64;
const TTL: Duration = Duration::from_secs(300);
const MAX_ARGUMENT_BYTES: usize = 65_536;

#[derive(Debug)]
struct Pending {
    operation: String,
    arguments: Value,
    issued: Instant,
}

/// A token approves one exact request, once, within five minutes.
/// The gate records metadata only; issuing or checking tokens performs no IO.
#[derive(Debug)]
pub struct ConfirmationGate {
    pending: Mutex<HashMap<String, Pending>>,
    random: std::collections::hash_map::RandomState,
    sequence: AtomicU64,
}

impl Default for ConfirmationGate {
    fn default() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            random: std::collections::hash_map::RandomState::new(),
            sequence: AtomicU64::new(0),
        }
    }
}

impl ConfirmationGate {
    pub fn check(
        &self,
        operation: &str,
        arguments: &Value,
        token: Option<&str>,
        cost: Value,
    ) -> Result<(), ToolError> {
        // Bound retained JSON as well as token count; malformed huge requests must
        // not turn a metadata-only preview into unbounded session storage.
        let bytes = serde_json::to_vec(arguments).map_err(|error| {
            ToolError::protocol(
                "invalid_params",
                format!("cannot encode arguments: {error}"),
            )
        })?;
        if bytes.len() > MAX_ARGUMENT_BYTES {
            return Err(ToolError::protocol(
                "confirmation_arguments_too_large",
                "confirmation arguments exceed 65536 normalized JSON bytes",
            ));
        }
        let mut pending = self.pending.lock().map_err(|_| {
            ToolError::protocol("confirmation_unavailable", "confirmation state unavailable")
        })?;
        pending.retain(|_, request| request.issued.elapsed() < TTL);
        if let Some(token) = token {
            let valid = pending.get(token).is_some_and(|request| {
                request.operation == operation && request.arguments == *arguments
            });
            if !valid {
                return Err(ToolError::protocol(
                    "invalid_confirmation",
                    "unknown, expired, consumed, or mismatched confirmation token",
                ));
            }
            pending.remove(token);
            return Ok(());
        }
        if pending.len() >= CAPACITY {
            return Err(ToolError::domain(
                "confirmation_capacity",
                "64 confirmations are pending; confirm an existing request or wait for expiry",
                json!({"capacity": CAPACITY, "expires_in_seconds": TTL.as_secs()}),
            ));
        }
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut first = self.random.build_hasher();
        first.write_u64(sequence);
        let mut second = self.random.build_hasher();
        second.write_u64(sequence);
        second.write_u8(1);
        let token = format!("{:016x}{:016x}", first.finish(), second.finish());
        pending.insert(
            token.clone(),
            Pending {
                operation: operation.into(),
                arguments: arguments.clone(),
                issued: Instant::now(),
            },
        );
        Err(ToolError::domain(
            "confirmation_required",
            "confirm this exact request before content IO",
            json!({
                "operation": operation,
                "confirmation_token": token,
                "cost": cost,
                "expires_in_seconds": TTL.as_secs(),
            }),
        ))
    }
}

/// Conservative decoded-source estimates from the immutable index alone.
/// These are planning estimates, not promises about physical IO or CPU time.
pub(crate) fn tool_cost(ctx: &Archive, operation: &str, arguments: &Value) -> Value {
    let entry_size = arguments["entry_id"]
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .and_then(|id| ctx.entry(EntryId(id)))
        .map(|entry| entry.uncompressed_size);
    let (stored, source, detail) = match operation {
        "archive_info" => (
            if arguments["with_sha256"].as_bool() == Some(false) {
                0
            } else {
                ctx.index().archive_file_size()
            },
            2 * (caps::MAX_FORMAT_VERSION_BYTES as u64 + 1),
            "May hash the entire stored ZIP unless cached or declined; marker reads repeat. Each marker accepts 256 decoded bytes with one overflow probe.",
        ),
        "entry_info" => (
            0,
            caps::DETECT_SAMPLE_BYTES,
            "Reads up to 8 KiB decoded detection sample with a fresh ZIP parser; directories or invalid entries may fail without a sample.",
        ),
        "find_by_type" => (
            0,
            ctx.entries()
                .iter()
                .filter(|entry| !entry.is_dir)
                .map(|_| caps::DETECT_SAMPLE_BYTES)
                .sum(),
            "Attempts a sample per file; the match-output limit does not limit classification work. Each sample is capped at 8 KiB; estimate allows the full cap for every file.",
        ),
        "read_entry" => (
            0,
            caps::MAX_ENTRY_BYTES + 1,
            "Late line or byte offsets require prefix decompression. The 512-byte excerpt cap bounds output, not reading; the entry guard may consume one overflow probe byte.",
        ),
        "grep_archive" => (
            0,
            arguments["scan_budget"]
                .as_u64()
                .unwrap_or(caps::MAX_GREP_SCAN_BYTES)
                .min(caps::MAX_GREP_SCAN_BYTES),
            "One page consumes up to scan_budget new decoded source bytes; continuation preserves its decoder. CPU, decoder buffering, and physical IO are not bounded by this estimate.",
        ),
        _ => (
            0,
            0,
            "No archive-content estimate is available for this operation.",
        ),
    };
    json!({
        "stored_bytes_upper_estimate": stored,
        "source_bytes_upper_estimate": source,
        "entry_declared_size_bytes": entry_size,
        "metadata_io": true,
        "estimate_basis": "immutable ZIP index and request limits; physical IO and CPU time may differ",
        "notes": [detail],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ToolErrorKind;

    fn issue(gate: &ConfirmationGate, operation: &str, arguments: &Value) -> String {
        let error = gate
            .check(operation, arguments, None, json!({}))
            .unwrap_err();
        assert_eq!(error.kind, ToolErrorKind::Domain);
        assert_eq!(error.code, "confirmation_required");
        error.data.unwrap()["confirmation_token"]
            .as_str()
            .unwrap()
            .into()
    }

    #[test]
    fn tokens_bind_operation_and_arguments_without_consuming_on_mismatch() {
        let gate = ConfirmationGate::default();
        let arguments = json!({"entry_id": 7, "byte_start": 0, "byte_end": 10});
        let token = issue(&gate, "read_entry", &arguments);
        for (operation, arguments) in [
            ("entry_info", arguments.clone()),
            (
                "read_entry",
                json!({"entry_id": 8, "byte_start": 0, "byte_end": 10}),
            ),
        ] {
            let error = gate
                .check(operation, &arguments, Some(&token), json!({}))
                .unwrap_err();
            assert_eq!(error.kind, ToolErrorKind::Protocol);
            assert_eq!(error.code, "invalid_confirmation");
        }
        gate.check("read_entry", &arguments, Some(&token), json!({}))
            .unwrap();
        assert_eq!(
            gate.check("read_entry", &arguments, Some(&token), json!({}))
                .unwrap_err()
                .code,
            "invalid_confirmation"
        );
    }

    #[test]
    fn tokens_expire_and_capacity_is_bounded() {
        let gate = ConfirmationGate::default();
        let arguments = json!({});
        let token = issue(&gate, "archive_info", &arguments);
        gate.pending.lock().unwrap().get_mut(&token).unwrap().issued = Instant::now() - TTL;
        assert_eq!(
            gate.check("archive_info", &arguments, Some(&token), json!({}))
                .unwrap_err()
                .code,
            "invalid_confirmation"
        );
        for _ in 0..CAPACITY {
            issue(&gate, "archive_info", &arguments);
        }
        assert_eq!(
            gate.check("archive_info", &arguments, None, json!({}))
                .unwrap_err()
                .code,
            "confirmation_capacity"
        );
        assert_eq!(gate.pending.lock().unwrap().len(), CAPACITY);
    }

    #[test]
    fn oversized_arguments_do_not_retain_state_or_consume_existing_approval() {
        let gate = ConfirmationGate::default();
        let arguments = json!({"pattern": "small"});
        let token = issue(&gate, "grep_archive", &arguments);
        let oversized = json!({"pattern": "x".repeat(MAX_ARGUMENT_BYTES)});
        for token in [None, Some(token.as_str())] {
            assert_eq!(
                gate.check("grep_archive", &oversized, token, json!({}))
                    .unwrap_err()
                    .code,
                "confirmation_arguments_too_large"
            );
            assert_eq!(gate.pending.lock().unwrap().len(), 1);
        }
        gate.check("grep_archive", &arguments, Some(&token), json!({}))
            .unwrap();
    }

    #[test]
    fn tokens_do_not_cross_gate_instances() {
        let first = ConfirmationGate::default();
        let second = ConfirmationGate::default();
        let arguments = json!({});
        let token = issue(&first, "archive_info", &arguments);
        assert_eq!(
            second
                .check("archive_info", &arguments, Some(&token), json!({}))
                .unwrap_err()
                .code,
            "invalid_confirmation"
        );
        first
            .check("archive_info", &arguments, Some(&token), json!({}))
            .unwrap();
    }
}
