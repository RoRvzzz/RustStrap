/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventHook {
    pub event_min: u32,
    pub event_max: u32,
    pub process_id: Option<u32>,
    pub thread_id: Option<u32>,
    pub out_of_context: bool,
}

pub trait EventBackend {
    fn register(&self, hook: EventHook) -> Result<u64>;

    fn unregister(&self, handle: u64) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsEventBackend;

static NEXT_EVENT_HANDLE: AtomicU64 = AtomicU64::new(1);
static EVENT_REGISTRY: Lazy<Mutex<HashMap<u64, EventHook>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

impl EventBackend for WindowsEventBackend {
    fn register(&self, hook: EventHook) -> Result<u64> {
        if hook.event_min > hook.event_max {
            return Err(anyhow!("event_min must be <= event_max"));
        }

        let handle = NEXT_EVENT_HANDLE.fetch_add(1, Ordering::Relaxed);
        EVENT_REGISTRY
            .lock()
            .map_err(|_| anyhow!("event registry poisoned"))?
            .insert(handle, hook);
        Ok(handle)
    }

    fn unregister(&self, handle: u64) -> Result<()> {
        EVENT_REGISTRY
            .lock()
            .map_err(|_| anyhow!("event registry poisoned"))?
            .remove(&handle);
        Ok(())
    }
}
