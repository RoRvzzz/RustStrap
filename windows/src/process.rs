use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessOptions {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: BTreeMap<String, String>,
    pub inherit_console: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessHandle {
    pub process_id: u32,
}

pub trait ProcessBackend {
    fn spawn(&self, options: ProcessOptions) -> Result<ProcessHandle>;

    fn is_running(&self, process_id: u32) -> Result<bool>;

    fn terminate(&self, process_id: u32) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsProcessBackend;

#[cfg(windows)]
impl ProcessBackend for WindowsProcessBackend {
    fn spawn(&self, options: ProcessOptions) -> Result<ProcessHandle> {
        use std::os::windows::process::CommandExt;

        let mut command = Command::new(&options.program);
        command.args(&options.arguments);

        if let Some(working_directory) = options.working_directory {
            command.current_dir(working_directory);
        }

        for (key, value) in options.environment {
            command.env(key, value);
        }

        if !options.inherit_console {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command.spawn()?;
        Ok(ProcessHandle {
            process_id: child.id(),
        })
    }

    fn is_running(&self, process_id: u32) -> Result<bool> {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {process_id}"), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("tasklist returned non-zero status"));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(&process_id.to_string()) && !stdout.contains("No tasks are running"))
    }

    fn terminate(&self, process_id: u32) -> Result<()> {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let status = Command::new("taskkill")
            .args(["/PID", &process_id.to_string(), "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status()?;
        if !status.success() {
            return Err(anyhow!("taskkill returned non-zero status"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
impl ProcessBackend for WindowsProcessBackend {
    fn spawn(&self, _options: ProcessOptions) -> Result<ProcessHandle> {
        Ok(ProcessHandle { process_id: 0 })
    }

    fn is_running(&self, _process_id: u32) -> Result<bool> {
        Ok(false)
    }

    fn terminate(&self, _process_id: u32) -> Result<()> {
        Ok(())
    }
}
