//! macOS Seatbelt (`sandbox-exec`) provider.

use anyhow::Result;
use tokio::process::Command;

use super::{apply_common, SandboxProfile, SandboxProvider};

/// Sandbox provider that uses the macOS `sandbox-exec` (Seatbelt) subsystem.
///
/// Generates a Seatbelt profile string that restricts filesystem writes to the
/// project directory and temp paths, limits network egress to localhost and
/// whitelisted hosts, and blocks all inbound connections.
pub struct SeatbeltProvider;

impl SandboxProvider for SeatbeltProvider {
    fn sandboxed_command(
        &self,
        binary: &str,
        args: &[&str],
        profile: &SandboxProfile,
    ) -> Result<Command> {
        let seatbelt = generate_seatbelt_profile(profile);

        let mut cmd = Command::new("sandbox-exec");
        cmd.arg("-p").arg(&seatbelt).arg(binary);
        for arg in args {
            cmd.arg(arg);
        }

        apply_common(&mut cmd, profile);
        Ok(cmd)
    }

    fn is_available(&self) -> bool {
        std::path::Path::new("/usr/bin/sandbox-exec").exists()
    }

    fn name(&self) -> &'static str {
        "seatbelt"
    }
}

/// Generate a macOS Seatbelt profile from a [`SandboxProfile`].
fn generate_seatbelt_profile(profile: &SandboxProfile) -> String {
    let project_dir = &profile.project_dir;

    format!(
        r#"(version 1)

;; Default: deny everything
(deny default)

;; Allow basic process operations
(allow process-exec)
(allow process-fork)
(allow signal)
(allow sysctl-read)

;; Allow file reads globally (needed for binary execution, libs, etc.)
(allow file-read*)

;; Allow file writes ONLY in project directory and temp
(allow file-write*
    (subpath "{project_dir}")
    (subpath "/tmp")
    (subpath "/private/tmp")
    (subpath "/var/folders")
)

;; Allow home dir dotfiles for tool configs
(allow file-write*
    (subpath (string-append (param "HOME") "/.config"))
    (subpath (string-append (param "HOME") "/.local"))
    (subpath (string-append (param "HOME") "/.cache"))
)

;; NETWORK: Allow ONLY outbound to localhost and whitelisted hosts
(allow network-outbound
    (remote ip "localhost:*")
    (remote unix-socket)
)

;; Allow DNS resolution
(allow network-outbound (remote ip "*:53"))

;; Allow connections to whitelisted APIs (HTTPS)
(allow network-outbound (remote ip "*:443"))

;; BLOCK all inbound network connections (no reverse shells)
(deny network-inbound)

;; Allow IPC (needed for stdio communication)
(allow ipc-posix-shm-read*)
(allow ipc-posix-shm-write*)
(allow mach-lookup)

;; Allow reading system info
(allow system-info)
"#
    )
}
