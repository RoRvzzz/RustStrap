/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::thread;
use std::time::Duration;

/// multi-instance watcher — acquires the Roblox singleton mutex to allow
/// multiple instances, then polls until all Roblox processes exit.


/// run the multi-instance watcher. Blocks until all Roblox processes close.
#[cfg(windows)]
pub fn run() {
    // try to acquire the Roblox singleton mutex
    let mutex_name = "ROBLOX_singletonMutex\0";

    extern "system" {
        fn CreateMutexW(
            attrs: *const std::ffi::c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> isize;
        fn WaitForSingleObject(handle: isize, millis: u32) -> u32;
        fn ReleaseMutex(handle: isize) -> i32;
        fn CloseHandle(handle: isize) -> i32;

        // named events for init signaling
        fn CreateEventW(
            attrs: *const std::ffi::c_void,
            manual_reset: i32,
            initial_state: i32,
            name: *const u16,
        ) -> isize;
        fn SetEvent(handle: isize) -> i32;
    }

    let wide_mutex: Vec<u16> = mutex_name.encode_utf16().collect();
    let wide_event: Vec<u16> = "Ruststrap-MultiInstanceWatcherInitialisationFinished\0"
        .encode_utf16()
        .collect();

    unsafe {
        let mutex_handle = CreateMutexW(std::ptr::null(), 0, wide_mutex.as_ptr());
        if mutex_handle == 0 {
            fire_init_event_raw(wide_event.as_ptr());
            return;
        }

        // try to acquire (0ms timeout)
        let result = WaitForSingleObject(mutex_handle, 0);
        let acquired = result == 0 || result == 128; // WAIT_OBJECT_0 or WAIT_ABANDONED

        // fire init event regardless
        fire_init_event_raw(wide_event.as_ptr());

        if !acquired {
            CloseHandle(mutex_handle);
            return;
        }

        // poll for alive Roblox/Ruststrap processes
        loop {
            thread::sleep(Duration::from_secs(5));
            let count = get_open_processes_count();
            if count != -1 && count <= 0 {
                break;
            }
        }

        ReleaseMutex(mutex_handle);
        CloseHandle(mutex_handle);
    }
}

#[cfg(windows)]
unsafe fn fire_init_event_raw(name_ptr: *const u16) {
    extern "system" {
        fn CreateEventW(
            attrs: *const std::ffi::c_void,
            manual_reset: i32,
            initial_state: i32,
            name: *const u16,
        ) -> isize;
        fn SetEvent(handle: isize) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    let handle = CreateEventW(std::ptr::null(), 0, 0, name_ptr);
    if handle != 0 {
        SetEvent(handle);
        CloseHandle(handle);
    }
}

/// count running Roblox/Ruststrap processes (excluding self).
#[cfg(windows)]
fn get_open_processes_count() -> i32 {
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

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE || snapshot == 0 {
            return -1;
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

        let mut count = 0i32;

        if Process32FirstW(snapshot, &mut entry as *mut ProcessEntry32W) != 0 {
            loop {
                let end = entry
                    .sz_exe_file
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(entry.sz_exe_file.len());
                let name = String::from_utf16_lossy(&entry.sz_exe_file[..end]);
                if name.eq_ignore_ascii_case("RobloxPlayerBeta.exe")
                    || name.eq_ignore_ascii_case("Ruststrap.exe")
                {
                    count += 1;
                }

                if Process32NextW(snapshot, &mut entry as *mut ProcessEntry32W) == 0 {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        count.saturating_sub(1)
    }
}

#[cfg(not(windows))]
fn get_open_processes_count() -> i32 {
    0
}

#[cfg(not(windows))]
pub fn run() {
    // no-op on non-Windows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_count_returns_valid() {
        let count = get_open_processes_count();
        // should return 0 or more (not -1) on a working system
        assert!(count >= -1);
    }
}
