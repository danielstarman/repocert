use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use wait_timeout::ChildExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommandRunnerOptions {
    pub argv: Vec<String>,
    pub env: std::collections::BTreeMap<String, String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CommandRunnerStatus {
    Exited { exit_code: Option<i32> },
    TimedOut,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommandExecution {
    pub status: CommandRunnerStatus,
    pub duration_ms: u64,
    pub message: Option<String>,
}

pub(crate) fn run_command(repo_root: &Path, options: &CommandRunnerOptions) -> CommandExecution {
    let start = Instant::now();

    let mut command = Command::new(&options.argv[0]);
    command.args(&options.argv[1..]);
    command.current_dir(repo_root);
    command.envs(&options.env);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return finish_execution(
                CommandRunnerStatus::Exited { exit_code: None },
                Some(format!("failed to spawn command: {error}")),
                start,
            );
        }
    };

    let stdout_handle = child.stdout.take().map(forward_to_parent_stderr);
    let stderr_handle = child.stderr.take().map(forward_to_parent_stderr);

    let wait_result = wait_for_child(&mut child, options.timeout_ms);

    let stdout_error = join_forwarder(stdout_handle);
    let stderr_error = join_forwarder(stderr_handle);

    match wait_result {
        Ok(WaitStatus::Exited(status)) => {
            if let Some(error) = stdout_error.or(stderr_error) {
                return finish_execution(
                    CommandRunnerStatus::Exited {
                        exit_code: status.code(),
                    },
                    Some(format!("failed to forward child output: {error}")),
                    start,
                );
            }

            finish_execution(
                CommandRunnerStatus::Exited {
                    exit_code: status.code(),
                },
                None,
                start,
            )
        }
        Ok(WaitStatus::TimedOut) => finish_execution(
            CommandRunnerStatus::TimedOut,
            options
                .timeout_ms
                .map(|timeout| format!("command timed out after {timeout} ms")),
            start,
        ),
        Err(error) => finish_execution(
            CommandRunnerStatus::Exited { exit_code: None },
            Some(format!("failed while waiting for command: {error}")),
            start,
        ),
    }
}

enum WaitStatus {
    Exited(std::process::ExitStatus),
    TimedOut,
}

fn wait_for_child(
    child: &mut std::process::Child,
    timeout_ms: Option<u64>,
) -> io::Result<WaitStatus> {
    match timeout_ms {
        Some(timeout_ms) => match child.wait_timeout(Duration::from_millis(timeout_ms))? {
            Some(status) => Ok(WaitStatus::Exited(status)),
            None => {
                child.kill()?;
                let _ = child.wait();
                Ok(WaitStatus::TimedOut)
            }
        },
        None => child.wait().map(WaitStatus::Exited),
    }
}

fn forward_to_parent_stderr<R: Read + Send + 'static>(
    mut reader: R,
) -> thread::JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        let mut stderr = io::stderr();
        let mut buffer = [0u8; 8192];

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            stderr.write_all(&buffer[..read])?;
            stderr.flush()?;
        }

        Ok(())
    })
}

fn join_forwarder(handle: Option<thread::JoinHandle<io::Result<()>>>) -> Option<String> {
    handle.and_then(|handle| match handle.join() {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("child output forwarding thread panicked".to_string()),
    })
}

fn finish_execution(
    status: CommandRunnerStatus,
    message: Option<String>,
    start: Instant,
) -> CommandExecution {
    CommandExecution {
        status,
        duration_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        message,
    }
}
