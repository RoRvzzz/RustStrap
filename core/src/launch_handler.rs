use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::Result;
use crate::process_utils::configure_hidden;

pub fn check_wmf_available() -> bool {
    let system = std::env::var("SYSTEMROOT").unwrap_or_else(|_| r"C:\Windows".to_string());
    let mfplat = Path::new(&system).join("System32").join("mfplat.dll");
    mfplat.exists()
}

pub fn is_roblox_running() -> bool {
    #[cfg(windows)]
    {
        for name in running_process_names() {
            if name.eq_ignore_ascii_case("RobloxPlayerBeta.exe") {
                return true;
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        false
    }
}

pub fn launch_background_updater(exe_path: &Path) -> Result<()> {
    let mut command = Command::new(exe_path);
    configure_hidden(&mut command);
    command.arg("-backgroundupdater").spawn().map_err(|e| {
        crate::errors::DomainError::Process(format!("failed to launch background updater: {e}"))
    })?;
    Ok(())
}

pub fn launch_watcher_process(exe_path: &Path, watcher_data: &str) -> Result<()> {
    let mut command = Command::new(exe_path);
    configure_hidden(&mut command);
    command
        .arg("-watcher")
        .arg(watcher_data)
        .spawn()
        .map_err(|e| {
            crate::errors::DomainError::Process(format!("failed to launch watcher: {e}"))
        })?;
    Ok(())
}

/* 

lets hope this works, currently I don't believe this is windows. Just my bad coding

*/

pub fn launch_trayhost_process(exe_path: &Path, watcher_data: &str) -> Result<()> {
    let resolved_exe = std::fs::canonicalize(exe_path).unwrap_or_else(|_| exe_path.to_path_buf());

    let mut command = Command::new(&resolved_exe);
    if let Some(parent) = resolved_exe.parent() {
        command.current_dir(parent);
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    configure_hidden(&mut command);

    let mut child = command
        .arg("-trayhost")
        .arg(watcher_data)
        .spawn()
        .map_err(|e| {
            crate::errors::DomainError::Process(format!("trayhost failed!: {e}"))
        })?;

    if std::env::var_os("RUSTSTRAP_DEBUG_TRAYHOST").is_some() {
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Some(status) = child.try_wait().map_err(|e| {
            crate::errors::DomainError::Process(format!("trayhost failed!: {e}"))
        })? {
            return Err(crate::errors::DomainError::Process(format!(
                "trayhost exited early with status: {status}"
            )));
        }
    }

    Ok(())
}



pub fn launch_multi_instance_watcher(exe_path: &Path) -> Result<()> {
    let mut command = Command::new(exe_path);
    configure_hidden(&mut command);
    command.arg("-multiinstancewatcher").spawn().map_err(|e| {
        crate::errors::DomainError::Process(format!("failed to launch multi-instance watcher: {e}"))
    })?;
    Ok(())
}

pub fn launch_settings(exe_path: &Path) -> Result<()> {
    let mut command = Command::new(exe_path);
    configure_hidden(&mut command);
    command.arg("-menu").spawn().map_err(|e| {
        crate::errors::DomainError::Process(format!("failed to launch settings: {e}"))
    })?;
    Ok(())
}

pub fn open_url(url: &str) -> Result<()> {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        configure_hidden(&mut command);
        command
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(url)
            .spawn()
            .map_err(|e| crate::errors::DomainError::Process(format!("failed to open url: {e}")))?;
    }

    #[cfg(not(windows))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| crate::errors::DomainError::Io(format!("failed to open url: {e}")))?;
    }

    Ok(())
}

#[cfg(windows)]
pub fn kill_background_updater() {
    extern "system" {
        fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> isize;
        fn SetEvent(handle: isize) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    const EVENT_MODIFY_STATE: u32 = 0x0002;

    let name: Vec<u16> = "Ruststrap-BackgroundUpdaterKillEvent\0"
        .encode_utf16()
        .collect();

    unsafe {
        let handle = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if handle != 0 {
            SetEvent(handle);
            CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
pub fn kill_background_updater() {}

#[cfg(windows)]
fn running_process_names() -> Vec<String> {
    #[repr(C)]
    struct ProcessEntry32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    const INVALID_HANDLE_VALUE: isize = -1;

    let mut names = Vec::<String>::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE || snapshot == 0 {
            return names;
        }

        let mut entry = ProcessEntry32W {
            dw_size: std::mem::size_of::<ProcessEntry32W>() as u32,
            cnt_usage: 0,
            th32_process_id: 0,
            th32_default_heap_id: 0,
            th32_module_id: 0,
            cnt_threads: 0,
            th32_parent_process_id: 0,
            pc_pri_class_base: 0,
            dw_flags: 0,
            sz_exe_file: [0; 260],
        };

        if Process32FirstW(snapshot, &mut entry as *mut ProcessEntry32W) != 0 {
            loop {
                let end = entry
                    .sz_exe_file
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(entry.sz_exe_file.len());
                let name = String::from_utf16_lossy(&entry.sz_exe_file[..end]);
                names.push(name);

                if Process32NextW(snapshot, &mut entry as *mut ProcessEntry32W) == 0 {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmf_check() {
        let _ = check_wmf_available();
    }
}
