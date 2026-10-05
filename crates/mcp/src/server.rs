//! Server lifecycle: open the archive, then serve stdio.
//!
//! The ordering is the point. [`bugreport_insight_core::Archive::open`] runs
//! **before** the transport starts, so a server that cannot see its archive
//! never answers `archive_info` — it exits non-zero instead. That is the exact
//! failure `REQ-MCP-08` exists to prevent, and the one place where "no result"
//! must not read as "empty result"
//! (`design/05-mcp-server-design.md` §2).

use bugreport_insight_core::{Archive, ArchiveError};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::adapter::ArchiveServer;
use crate::diag;

/// Opens `path` and serves MCP over stdio until stdin closes.
///
/// EOF on stdin is **exit 0**, not an error: that is what a client disconnect
/// actually looks like, and treating it as a failure produces false errors in
/// every agent harness.
pub async fn serve(path: &std::path::Path) -> Result<(), ArchiveError> {
    let archive = Archive::open(path)?;
    diag::info(&format!(
        "opened {} — {} entries",
        path.display(),
        archive.entry_count()
    ));

    let ctx = std::sync::Arc::new(archive);
    let service = ArchiveServer::new(ctx)
        .serve(stdio())
        .await
        .map_err(|e| ArchiveError::Open {
            path: path.to_path_buf(),
            source: std::io::Error::other(e.to_string()),
        })?;

    match service.waiting().await {
        Ok(_) => {
            diag::info("stdin closed; shutting down");
            Ok(())
        }
        Err(e) => {
            diag::error(&format!("transport task failed: {e}"));
            Ok(())
        }
    }
}
