//! git/cargo-style subcommand dispatch: `bugreport-insight <verb> [args...]` execs a sibling
//! `bugreport-insight-<verb>` binary rather than being implemented inline. Built-in verbs
//! (`anr`, `apps`, `stacktrace`, `memory`, `cpu`) ship as separate binaries in this workspace;
//! any other verb (for example a third-party `dumpstate` subcommand) is looked up the same way,
//! so it only runs if a matching `bugreport-insight-<verb>` executable is installed on `PATH`.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

/// Verbs implemented as bundled binaries in this workspace. Always dispatched as a subcommand
/// (never mistaken for a `BUGREPORT_ZIP` path), even if the plugin binary happens to be missing
/// from this build.
pub const BUILTIN_SUBCOMMANDS: &[&str] = &["anr", "apps", "stacktrace", "memory", "cpu"];

fn plugin_name(verb: &str) -> String {
    format!("bugreport-insight-{verb}{}", env::consts::EXE_SUFFIX)
}

/// Looks for `bugreport-insight-<verb>` next to the current executable (so a dev build or an
/// installed bundle finds its own sibling binaries first), then on `PATH` (where a third-party
/// subcommand, or a manually installed built-in one, would live).
fn find_plugin(verb: &str) -> Option<PathBuf> {
    let name = plugin_name(verb);
    let beside_current_exe = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|dir| dir.join(&name))
        .filter(|path| path.is_file());
    beside_current_exe.or_else(|| {
        env::var_os("PATH").and_then(|path_var| {
            env::split_paths(&path_var)
                .map(|dir| dir.join(&name))
                .find(|path| path.is_file())
        })
    })
}

/// If `verb` names a built-in or discoverable subcommand, execs its plugin binary with
/// `args` (inheriting stdio) and returns its exit code. Returns `None` when `verb` is not a
/// built-in and no matching plugin binary can be found, so the caller can fall back to the
/// default full-report behavior (treating `verb` as a `BUGREPORT_ZIP` path).
pub fn dispatch(verb: &str, args: &[OsString]) -> Option<ExitCode> {
    let is_builtin = BUILTIN_SUBCOMMANDS.contains(&verb);
    let Some(exe) = find_plugin(verb) else {
        if is_builtin {
            eprintln!(
                "error: built-in subcommand `{verb}` is not available (missing `{}`; is this workspace fully built?)",
                plugin_name(verb)
            );
            return Some(ExitCode::FAILURE);
        }
        return None;
    };
    match Command::new(exe).args(args).status() {
        Ok(status) => Some(ExitCode::from(status.code().unwrap_or(1) as u8)),
        Err(error) => {
            eprintln!("error: failed to run subcommand `{verb}`: {error}");
            Some(ExitCode::FAILURE)
        }
    }
}
