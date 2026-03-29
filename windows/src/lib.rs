/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
#![forbid(unsafe_code)]

mod events;
mod mutex;
mod process;
mod registry;
mod shell;
mod shortcuts;

pub use anyhow::{anyhow, Result};

pub use events::{EventBackend, EventHook, WindowsEventBackend};
pub use mutex::{MutexBackend, NamedMutex, WindowsMutexBackend};
pub use process::{ProcessBackend, ProcessHandle, ProcessOptions, WindowsProcessBackend};
pub use registry::{RegistryBackend, RegistryHive, RegistryValue, WindowsRegistryBackend};
pub use shell::{ShellBackend, WindowsShellBackend};
pub use shortcuts::{ShortcutBackend, ShortcutRequest, WindowsShortcutBackend};

pub trait PlatformBackend:
    RegistryBackend + ProcessBackend + ShellBackend + MutexBackend + EventBackend + ShortcutBackend
{
}

impl<T> PlatformBackend for T where
    T: RegistryBackend
        + ProcessBackend
        + ShellBackend
        + MutexBackend
        + EventBackend
        + ShortcutBackend
{
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_backends_execute_real_operations() {
        let registry = WindowsRegistryBackend::default();
        let process = WindowsProcessBackend::default();
        let mutex = WindowsMutexBackend::default();
        let events = WindowsEventBackend::default();
        let shortcuts = WindowsShortcutBackend::default();

        let key_path = r"Software\Ruststrap\PlatformWindowsTests";
        let set_result = registry.set_value(
            RegistryHive::CurrentUser,
            key_path,
            "Test",
            RegistryValue::String("value".into()),
        );
        if let Err(error) = set_result {
            if error.to_string().contains("Access is denied") {
                // restricted sandboxes may block registry writes.
                return;
            }
            panic!("set registry value: {error}");
        }

        let value = registry
            .get_value(RegistryHive::CurrentUser, key_path, "Test")
            .expect("get registry value");
        assert_eq!(value, Some(RegistryValue::String("value".into())));

        let handle = process
            .spawn(ProcessOptions {
                program: "cmd.exe".into(),
                arguments: vec!["/C".into(), "timeout /T 1 /NOBREAK > NUL".into()],
                working_directory: None,
                environment: Default::default(),
                inherit_console: false,
            })
            .expect("spawn process");
        assert!(process
            .is_running(handle.process_id)
            .expect("check process running"));
        process
            .terminate(handle.process_id)
            .expect("terminate process");

        let named_mutex = mutex.create_named("Ruststrap-test").expect("create mutex");
        assert!(mutex.try_acquire(&named_mutex).expect("acquire mutex"));
        mutex.release(named_mutex).expect("release mutex");

        let hook_handle = events
            .register(EventHook {
                event_min: 1,
                event_max: 2,
                process_id: None,
                thread_id: None,
                out_of_context: true,
            })
            .expect("register event hook");
        events.unregister(hook_handle).expect("unregister hook");

        let temp = tempdir().expect("tempdir");
        let shortcut_path = temp.path().join("Ruststrap-test.lnk");
        shortcuts
            .create_shortcut(ShortcutRequest {
                shortcut_path: shortcut_path.clone(),
                target_path: std::path::PathBuf::from(r"C:\Windows\System32\notepad.exe"),
                arguments: Vec::new(),
                working_directory: None,
                icon_path: None,
                description: None,
            })
            .expect("create shortcut");
        assert!(shortcut_path.exists());
        shortcuts
            .remove_shortcut(shortcut_path.clone())
            .expect("remove shortcut");
        assert!(!shortcut_path.exists());

        registry
            .delete_key(RegistryHive::CurrentUser, key_path)
            .expect("cleanup registry");
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_backends_are_noops() {
        let registry = WindowsRegistryBackend::default();
        let shell = WindowsShellBackend::default();
        let process = WindowsProcessBackend::default();
        let mutex = WindowsMutexBackend::default();
        let events = WindowsEventBackend::default();
        let shortcuts = WindowsShortcutBackend::default();

        assert!(registry
            .set_value(
                RegistryHive::CurrentUser,
                r"Software\Ruststrap",
                "Test",
                RegistryValue::String("value".into()),
            )
            .is_ok());
        assert_eq!(
            registry
                .get_value(RegistryHive::CurrentUser, r"Software\Ruststrap", "Test")
                .unwrap(),
            None
        );

        assert!(shell.open_url("https://example.com").is_ok());
        assert!(process
            .spawn(ProcessOptions {
                program: "cmd.exe".into(),
                arguments: vec!["/C".into(), "exit".into()],
                working_directory: None,
                environment: Default::default(),
                inherit_console: false,
            })
            .is_ok());
        assert!(mutex.create_named("Ruststrap-test").is_ok());
        assert!(events
            .register(EventHook {
                event_min: 1,
                event_max: 2,
                process_id: None,
                thread_id: None,
                out_of_context: true,
            })
            .is_ok());
        assert!(shortcuts
            .create_shortcut(ShortcutRequest {
                shortcut_path: std::path::PathBuf::from("test.lnk"),
                target_path: std::path::PathBuf::from("target.exe"),
                arguments: Vec::new(),
                working_directory: None,
                icon_path: None,
                description: None,
            })
            .is_ok());
    }
}
