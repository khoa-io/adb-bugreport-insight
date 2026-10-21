//! Server lifecycle: open the archive, then serve stdio.
//!
//! The ordering is the point. [`bugreport_insight_core::Archive::open`] runs
//! **before** the transport starts, so a server that cannot see its archive
//! never answers `archive_info` — it exits non-zero instead. That is the exact
//! failure `REQ-MCP-08` exists to prevent, and the one place where "no result"
//! must not read as "empty result"
//! (`docs/swe.3-detailed-design/03-mcp-server.md`).

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bugreport_insight_core::{Archive, ArchiveError, ArchiveLimits};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::adapter::{ArchiveServer, InsightServer};
use crate::diag;

/// Failure starting or running the MCP service.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error("MCP stdio transport failed to start: {0}")]
    TransportStart(String),
    #[error("MCP server task failed: {0}")]
    ServerTask(String),
    #[error("MCP stdio transport failed: {0}")]
    TransportIo(#[from] io::Error),
}

/// Opens `path` and serves MCP over stdio until stdin closes.
///
/// EOF on stdin is **exit 0**, not an error: that is what a client disconnect
/// actually looks like, and treating it as a failure produces false errors in
/// every agent harness.
pub async fn serve(path: &std::path::Path) -> Result<(), ServeError> {
    serve_with_options(path, false, ArchiveLimits::DEFAULT).await
}

/// Serve archive tools and, when requested, in-memory insight tools.
///
/// Directory bindings are accepted only with insight enabled, and discovery
/// remains confined to immediate regular ZIP files in that directory.
pub async fn serve_with_options(
    path: &std::path::Path,
    insight: bool,
    limits: ArchiveLimits,
) -> Result<(), ServeError> {
    limits.validate()?;
    if insight {
        let archive = if path.is_dir() {
            None
        } else {
            Some(ArchiveServer::new(Arc::new(Archive::open_with(
                path, limits,
            )?)))
        };
        run(InsightServer::new(path, limits, archive)?).await
    } else {
        let archive = Archive::open_with(path, limits)?;
        diag::info(&format!(
            "opened {} — {} entries",
            path.display(),
            archive.entry_count()
        ));
        run(ArchiveServer::new(Arc::new(archive))).await
    }
}

async fn run(handler: impl rmcp::ServerHandler) -> Result<(), ServeError> {
    let service = match handler.serve(stdio()).await {
        Ok(service) => service,
        Err(error)
            if error
                .to_string()
                .contains("connection closed: initialize request") =>
        {
            diag::info("stdin closed before initialization; shutting down");
            return Ok(());
        }
        Err(error) => return Err(ServeError::TransportStart(error.to_string())),
    };
    supervise(service.waiting()).await
}

/// Supervise the SDK's real service future, including nested send-task failures.
async fn supervise(
    waiting: impl std::future::Future<
        Output = Result<rmcp::service::QuitReason, tokio::task::JoinError>,
    >,
) -> Result<(), ServeError> {
    let stop_monitor = Arc::new(AtomicBool::new(false));
    let monitor_signal = Arc::clone(&stop_monitor);
    let mut stdout_monitor = tokio::task::spawn_blocking(move || monitor_stdout(monitor_signal));

    let waiting = tokio::select! {
        result = &mut stdout_monitor => {
            return match result {
                Ok(Err(error)) => Err(ServeError::TransportIo(error)),
                Ok(Ok(())) => Err(ServeError::ServerTask(
                    "stdout monitor stopped unexpectedly".to_owned(),
                )),
                Err(error) => Err(ServeError::ServerTask(
                    format!("stdout monitor task failed: {error}"),
                )),
            };
        }
        result = waiting => result,
    };

    stop_monitor.store(true, Ordering::Release);
    match stdout_monitor.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(ServeError::TransportIo(error)),
        Err(error) => {
            return Err(ServeError::ServerTask(format!(
                "stdout monitor task failed: {error}"
            )));
        }
    }

    if stdout_is_disconnected()? {
        return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "stdout response pipe disconnected",
        )
        .into());
    }

    match waiting {
        Ok(rmcp::service::QuitReason::Closed | rmcp::service::QuitReason::Cancelled) => {
            diag::info("MCP service closed; shutting down");
            Ok(())
        }
        Ok(rmcp::service::QuitReason::JoinError(error)) | Err(error) => {
            Err(ServeError::ServerTask(error.to_string()))
        }
        Ok(reason) => Err(ServeError::ServerTask(format!(
            "unrecognized SDK service termination: {reason:?}"
        ))),
    }
}

#[cfg(unix)]
fn monitor_stdout(stop: Arc<AtomicBool>) -> io::Result<()> {
    while !stop.load(Ordering::Acquire) {
        if stdout_is_disconnected()? {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout response pipe disconnected",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    Ok(())
}

#[cfg(not(unix))]
fn monitor_stdout(stop: Arc<AtomicBool>) -> io::Result<()> {
    while !stop.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    Ok(())
}

#[cfg(unix)]
fn stdout_is_disconnected() -> io::Result<bool> {
    use std::os::fd::AsRawFd;

    let stdout = io::stdout();
    let mut descriptor = libc::pollfd {
        fd: stdout.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: `descriptor` is a valid pointer to one initialized pollfd.
    let result = unsafe { libc::poll(&mut descriptor, 1, 0) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(error);
    }
    Ok(descriptor.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0)
}

#[cfg(not(unix))]
fn stdout_is_disconnected() -> io::Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::RoleServer;
    use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage, serve_directly};
    use rmcp::transport::Transport;

    #[derive(Debug)]
    struct TestServer;
    impl rmcp::ServerHandler for TestServer {}

    struct FaultTransport {
        panic_receive: bool,
    }

    impl Transport<RoleServer> for FaultTransport {
        type Error = io::Error;

        fn send(
            &mut self,
            _: TxJsonRpcMessage<RoleServer>,
        ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
            std::future::poll_fn(|_| panic!("injected SDK send-task failure"))
        }

        async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
            assert!(!self.panic_receive, "injected SDK service-task failure");
            std::future::pending().await
        }

        async fn close(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn actual_sdk_service_task_panic_is_a_server_failure() {
        let service = serve_directly(
            TestServer,
            FaultTransport {
                panic_receive: true,
            },
            None,
        );
        let error = supervise(service.waiting()).await.unwrap_err();
        assert!(matches!(error, ServeError::ServerTask(_)), "{error}");
        assert!(
            error
                .to_string()
                .contains("injected SDK service-task failure")
        );
    }

    #[tokio::test]
    async fn actual_sdk_send_task_panic_is_a_server_failure() {
        let service = serve_directly(
            TestServer,
            FaultTransport {
                panic_receive: false,
            },
            None,
        );
        // This runs an SDK send task, whose panic is returned inside Ok(QuitReason).
        let peer = service.peer().clone();
        let sending = tokio::spawn(async move { peer.notify_tool_list_changed().await });
        let result = supervise(service.waiting()).await;
        assert!(sending.await.unwrap().is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ServeError::ServerTask(_)), "{error}");
        assert!(error.to_string().contains("injected SDK send-task failure"));
    }
}
