use crate::core::boundaries::VerificationGuestBoundary;
use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use crate::core::verification_model::{GuestCommandOutput, LibvirtGuestHandle};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestCommandRunner {
    pub ssh_binary: String,
    pub scp_binary: String,
}

impl Default for GuestCommandRunner {
    fn default() -> Self {
        Self {
            ssh_binary: "ssh".to_string(),
            scp_binary: "scp".to_string(),
        }
    }
}

impl GuestCommandRunner {
    pub fn ssh_command(&self, guest: &LibvirtGuestHandle, remote_command: &str) -> Command {
        let mut command = Command::new(&self.ssh_binary);
        command.args(common_ssh_options());
        command.arg(&guest.ssh_target).arg(remote_command);
        command
    }

    fn scp_command(
        &self,
        local_path: &Path,
        remote_target: &str,
        recursive: bool,
    ) -> Command {
        let mut command = Command::new(&self.scp_binary);
        command.args(common_ssh_options());
        if recursive {
            command.arg("-r");
        }
        command.arg(local_path).arg(remote_target);
        command
    }

    fn run_output(
        &self,
        command: &mut Command,
        context: &str,
    ) -> Result<GuestCommandOutput, CoreError> {
        self.run_output_with_timeout(command, context, None)
    }

    fn run_output_with_timeout(
        &self,
        command: &mut Command,
        context: &str,
        timeout: Option<Duration>,
    ) -> Result<GuestCommandOutput, CoreError> {
        let rendered = render_command(command);
        if let Some(timeout) = timeout {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = command.spawn().map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to launch {context}: {rendered}: {err}"),
                )
            })?;
            let deadline = Instant::now() + timeout;
            loop {
                if let Some(status) = child.try_wait().map_err(|err| {
                    CoreError::new(
                        FailureClass::Apply,
                        format!("failed to poll {context}: {rendered}: {err}"),
                    )
                })? {
                    let output = child.wait_with_output().map_err(|err| {
                        CoreError::new(
                            FailureClass::Apply,
                            format!("failed to collect {context} output: {rendered}: {err}"),
                        )
                    })?;
                    return Ok(GuestCommandOutput {
                        status_code: status.code().unwrap_or(1),
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CoreError::new(
                        FailureClass::Transient,
                        format!("{context} timed out after {}s: {rendered}", timeout.as_secs()),
                    ));
                }
                sleep(Duration::from_millis(100));
            }
        }
        let output = command.output().map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to launch {context}: {rendered}: {err}"),
            )
        })?;
        Ok(GuestCommandOutput {
            status_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

impl VerificationGuestBoundary for GuestCommandRunner {
    fn wait_ready(
        &self,
        guest: &LibvirtGuestHandle,
        timeout: &str,
    ) -> Result<GuestCommandOutput, CoreError> {
        if timeout.trim().is_empty() {
            return Err(CoreError::new(
                FailureClass::Validation,
                "wait_ready requires an explicit timeout",
            ));
        }
        if !guest.env_backed {
            return Ok(GuestCommandOutput {
                status_code: 0,
                stdout: format!("{} ready within {timeout}", guest.guest_name),
                stderr: String::new(),
            });
        }

        let duration = parse_timeout(timeout)?;
        let deadline = Instant::now() + duration;
        let mut consecutive_successes: u32 = 0;
        while Instant::now() < deadline {
            let mut command = self.ssh_command(guest, "true");
            let output = self.run_output(&mut command, "ssh readiness probe")?;
            if output.status_code == 0 {
                consecutive_successes += 1;
                if consecutive_successes >= 2 {
                    return Ok(GuestCommandOutput {
                        status_code: 0,
                        stdout: format!("{} ready within {timeout}", guest.ssh_target),
                        stderr: String::new(),
                    });
                }
            } else {
                consecutive_successes = 0;
            }
            sleep(Duration::from_secs(2));
        }
        Err(CoreError::new(
            FailureClass::Transient,
            format!("guest {} did not become ready within {timeout}", guest.ssh_target),
        ))
    }

    fn run_command(
        &self,
        guest: &LibvirtGuestHandle,
        command: &str,
        _timeout: Option<&str>,
    ) -> Result<GuestCommandOutput, CoreError> {
        if command.trim().is_empty() {
            return Err(CoreError::new(
                FailureClass::Validation,
                "guest command must not be empty",
            ));
        }
        if !guest.env_backed {
            return Ok(GuestCommandOutput {
                status_code: if command.contains("fail-step") { 1 } else { 0 },
                stdout: synthetic_guest_output(&guest.guest_name, command),
                stderr: String::new(),
            });
        }

        let mut ssh = self.ssh_command(guest, command);
        let timeout = _timeout.map(parse_timeout).transpose()?;
        self.run_output_with_timeout(&mut ssh, "ssh guest command", timeout)
    }

    fn copy_to_guest(
        &self,
        guest: &LibvirtGuestHandle,
        local_path: &Path,
        remote_path: &str,
        recursive: bool,
        executable: bool,
    ) -> Result<(), CoreError> {
        if !guest.env_backed {
            return Ok(());
        }
        let remote_target = format!("{}:{}", guest.ssh_target, remote_path);
        let mut scp = self.scp_command(local_path, &remote_target, recursive);
        let output = self.run_output(&mut scp, "scp guest copy")?;
        if output.status_code != 0 {
            return Err(CoreError::new(
                FailureClass::Apply,
                format!("failed to copy {} to guest: {}", local_path.display(), output.stderr),
            ));
        }
        if executable {
            let mut chmod_command =
                self.ssh_command(guest, &format!("chmod +x {}", shell_escape(remote_path)));
            let chmod = self.run_output(&mut chmod_command, "chmod guest file")?;
            if chmod.status_code != 0 {
                return Err(CoreError::new(
                    FailureClass::Apply,
                    format!("failed to mark {remote_path} executable: {}", chmod.stderr),
                ));
            }
        }
        Ok(())
    }
}

fn parse_timeout(timeout: &str) -> Result<Duration, CoreError> {
    let trimmed = timeout.trim();
    let seconds = trimmed
        .strip_suffix('s')
        .unwrap_or(trimmed)
        .parse::<u64>()
        .map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid timeout `{timeout}`: {err}"),
            )
        })?;
    Ok(Duration::from_secs(seconds))
}

fn synthetic_guest_output(guest_name: &str, command: &str) -> String {
    if command.contains(" apply") {
        if command.contains(" --json") {
            serde_json::json!({
                "command": "apply",
                "interface": "machine_readable",
                "guest": guest_name,
                "outcome": "converged",
                "changes": "none"
            })
            .to_string()
        } else if command.contains(" --verbose") {
            format!(
                "Apply for host {guest_name}\nExecution\n- simulated verbose apply\nSummary\n1 unchanged\nOutcome: converged\n"
            )
        } else {
            format!("Apply for host {guest_name}\nSummary\n1 unchanged\nOutcome: converged\n")
        }
    } else if command.contains(" explain") {
        if command.contains(" --json") {
            serde_json::json!({
                "command": "explain",
                "interface": "machine_readable",
                "guest": guest_name,
                "summary": "simulated"
            })
            .to_string()
        } else {
            format!("{guest_name}: explain simulated")
        }
    } else if command.contains(" plan") {
        if command.contains(" --json") {
            serde_json::json!({
                "command": "plan",
                "interface": "machine_readable",
                "guest": guest_name,
                "summary": "simulated"
            })
            .to_string()
        } else if command.contains(" --verbose") {
            format!("{guest_name}: verbose plan simulated")
        } else {
            format!("{guest_name}: plan simulated")
        }
    } else if command.contains(" status") {
        format!("{guest_name}: status simulated")
    } else if command.contains(" agent") {
        format!("{guest_name}: agent simulated")
    } else if command.contains("reboot") {
        format!("{guest_name}: reboot simulated")
    } else {
        format!("{guest_name}:{command}")
    }
}

fn common_ssh_options() -> [&'static str; 4] {
    [
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
    ]
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn render_command(command: &Command) -> String {
    let mut rendered = command.get_program().to_string_lossy().to_string();
    for arg in command.get_args() {
        rendered.push(' ');
        rendered.push_str(&shell_escape(&arg.to_string_lossy()));
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::GuestCommandRunner;

    #[test]
    fn run_output_with_timeout_returns_transient_error_for_hung_command() {
        let runner = GuestCommandRunner::default();
        let mut command = std::process::Command::new("bash");
        command.arg("-lc").arg("sleep 2");

        let err = runner
            .run_output_with_timeout(
                &mut command,
                "test command",
                Some(std::time::Duration::from_millis(100)),
            )
            .expect_err("timeout");

        assert_eq!(err.class, crate::core::types::FailureClass::Transient);
        assert!(err.message.contains("timed out"));
    }
}
