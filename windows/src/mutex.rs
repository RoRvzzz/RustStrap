/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use anyhow::{anyhow, Result};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedMutex {
    pub name: String,
    pub lock_path: PathBuf,
}

pub trait MutexBackend {
    fn create_named(&self, name: &str) -> Result<NamedMutex>;

    fn try_acquire(&self, mutex: &NamedMutex) -> Result<bool>;

    fn release(&self, mutex: NamedMutex) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsMutexBackend;

impl MutexBackend for WindowsMutexBackend {
    fn create_named(&self, name: &str) -> Result<NamedMutex> {
        let mut safe_name = name.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
        if safe_name.is_empty() {
            safe_name = "mutex".to_string();
        }

        let lock_root = std::env::temp_dir().join("Ruststrap-mutex");
        fs::create_dir_all(&lock_root)?;
        let lock_path = lock_root.join(format!("{safe_name}.lock"));

        Ok(NamedMutex {
            name: name.to_owned(),
            lock_path,
        })
    }

    fn try_acquire(&self, mutex: &NamedMutex) -> Result<bool> {
        let open_result = OpenOptions::new()
            .create(true)
            .create_new(true)
            .read(true)
            .write(true)
            .open(&mutex.lock_path);

        match open_result {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(err) => Err(anyhow!(err)),
        }
    }

    fn release(&self, mutex: NamedMutex) -> Result<()> {
        match fs::remove_file(mutex.lock_path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(anyhow!(err)),
        }
    }
}
