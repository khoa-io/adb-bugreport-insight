//! The tool seam: seven handlers, no protocol types.
//!
//! Nothing in this module — or anywhere in [`crate::tools`] — names an `rmcp`
//! symbol. That is what makes the whole tool layer, and the `REQ-MCP-08`
//! four-way status matrix, unit-testable with no server running.

use serde_json::Value;
use std::sync::Arc;

use crate::confirmation::{ConfirmationGate, tool_cost};

use crate::error::ToolError;
use crate::tools::envelope::ToolEnvelope;
use crate::tools::handlers::{self, Ctx};

pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn call(&self, params: Value) -> Result<ToolEnvelope, ToolError>;
}

/// Exactly seven handlers; their vector order does not define wire discovery order.
///
/// The adapter derives `tools/list` independently; wire tests verify its exact
/// name set and schemas. Every handler shares the same `Arc<Archive>`: one
/// immutable index; grep additionally owns bounded continuation state.
pub fn registry(ctx: Ctx) -> Vec<Box<dyn ToolHandler>> {
    registry_with_gate(ctx, Arc::new(ConfirmationGate::default()), None)
}

/// Build one archive's handlers on the same request-approval state.
pub(crate) fn registry_with_gate(
    ctx: Ctx,
    gate: Arc<ConfirmationGate>,
    confirmation_scope: Option<Value>,
) -> Vec<Box<dyn ToolHandler>> {
    let gated = |inner: Box<dyn ToolHandler>| -> Box<dyn ToolHandler> {
        Box::new(GatedTool {
            inner,
            ctx: ctx.clone(),
            gate: gate.clone(),
            confirmation_scope: confirmation_scope.clone(),
        })
    };
    vec![
        gated(Box::new(handlers::ArchiveInfo::new(ctx.clone()))),
        Box::new(handlers::ListEntries::new(ctx.clone())),
        Box::new(handlers::FindEntries::new(ctx.clone())),
        gated(Box::new(handlers::EntryInfo::new(ctx.clone()))),
        gated(Box::new(handlers::FindByType::new(ctx.clone()))),
        gated(Box::new(handlers::ReadEntry::new(ctx.clone()))),
        gated(Box::new(handlers::GrepArchive::new(ctx.clone()))),
    ]
}

/// Confirmation precedes handler execution, including search-cursor consumption.
struct GatedTool {
    inner: Box<dyn ToolHandler>,
    ctx: Ctx,
    gate: Arc<ConfirmationGate>,
    confirmation_scope: Option<Value>,
}

impl ToolHandler for GatedTool {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn description(&self) -> &'static str {
        self.inner.description()
    }

    fn call(&self, mut params: Value) -> Result<ToolEnvelope, ToolError> {
        let arguments = params.as_object_mut().ok_or_else(|| {
            ToolError::protocol("invalid_params", "tool arguments must be an object")
        })?;
        let token = match arguments.remove("confirmation") {
            None | Some(Value::Null) => None,
            Some(Value::String(token)) => Some(token),
            Some(_) => {
                return Err(ToolError::protocol(
                    "invalid_params",
                    "confirmation must be a string",
                ));
            }
        };
        let mut bound_arguments = params.clone();
        if let Some(scope) = &self.confirmation_scope
            && let Some(arguments) = bound_arguments.as_object_mut()
        {
            arguments.insert("_bugreport_insight_context".into(), scope.clone());
        }
        self.gate.check(
            self.name(),
            &bound_arguments,
            token.as_deref(),
            tool_cost(&self.ctx, self.name(), &params),
        )?;
        self.inner.call(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingTool(Arc<AtomicUsize>);
    impl ToolHandler for CountingTool {
        fn name(&self) -> &'static str {
            "read_entry"
        }
        fn description(&self) -> &'static str {
            "test"
        }
        fn call(&self, _: Value) -> Result<ToolEnvelope, ToolError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(ToolError::protocol("test_invoked", "counter incremented"))
        }
    }

    #[test]
    fn wrapped_handler_runs_only_after_exact_confirmation() {
        let root =
            std::env::temp_dir().join(format!("confirmation-wrapper-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("empty.zip");
        // Empty ZIP EOCD, sufficient to construct an immutable metadata context.
        std::fs::write(
            &path,
            [
                0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        )
        .unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let tool = GatedTool {
            inner: Box::new(CountingTool(count.clone())),
            ctx: Arc::new(bugreport_insight_core::Archive::open(&path).unwrap()),
            gate: Arc::new(ConfirmationGate::default()),
            confirmation_scope: None,
        };
        let mut arguments = json!({"entry_id":7,"byte_start":0,"byte_end":10});
        let error = tool.call(arguments.clone()).unwrap_err();
        assert_eq!(error.code, "confirmation_required");
        assert_eq!(count.load(Ordering::Relaxed), 0);
        arguments["confirmation"] = error.data.unwrap()["confirmation_token"].clone();
        assert_eq!(
            tool.call(arguments.clone()).unwrap_err().code,
            "test_invoked"
        );
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(
            tool.call(arguments).unwrap_err().code,
            "invalid_confirmation"
        );
        assert_eq!(count.load(Ordering::Relaxed), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
