/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use crate::{DomainError, DomainEvent, LaunchMode, PromptKind, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapStep {
    ConnectivityCheck,
    VersionResolution,
    DownloadPackages,
    ApplyModifications,
    RegisterSystemState,
    LaunchClient,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapContext {
    pub mode: LaunchMode,
    pub launch_args: String,
    pub quiet: bool,
    pub force_upgrade: bool,
    pub no_launch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    pub steps: Vec<BootstrapStep>,
    pub events: Vec<DomainEvent>,
}

pub trait BootstrapRuntime {
    fn prepare_launch_context(
        &self,
        _mode: LaunchMode,
        _launch_args: &str,
        _channel_override: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }

    fn check_connectivity(&self) -> Result<()>;
    fn resolve_version(&self, mode: LaunchMode, force_upgrade: bool) -> Result<String>;
    fn sync_packages(&self, version_guid: &str) -> Result<()>;
    fn apply_modifications(&self, version_guid: &str) -> Result<()>;
    fn register_system_state(&self, mode: LaunchMode, version_guid: &str) -> Result<()>;
    fn launch_client(&self, mode: LaunchMode, launch_args: &str) -> Result<u32>;
}

pub fn run_bootstrap_flow(
    runtime: &dyn BootstrapRuntime,
    context: &BootstrapContext,
) -> Result<BootstrapReport> {
    let mut sink = |_event: &DomainEvent| {};
    run_bootstrap_flow_with_observer(runtime, context, &mut sink)
}

pub fn run_bootstrap_flow_with_observer(
    runtime: &dyn BootstrapRuntime,
    context: &BootstrapContext,
    on_event: &mut dyn FnMut(&DomainEvent),
) -> Result<BootstrapReport> {
    let mut steps = Vec::new();
    let mut events = Vec::new();

    let mut record_event = |event: DomainEvent| {
        on_event(&event);
        events.push(event);
    };

    if context.mode == LaunchMode::None {
        return Err(DomainError::InvalidLaunchRequest(
            "launch mode cannot be none".to_string(),
        ));
    }

    record_event(DomainEvent::BootstrapStatus {
        message: "checking_connectivity".to_string(),
    });
    steps.push(BootstrapStep::ConnectivityCheck);
    runtime.check_connectivity()?;

    record_event(DomainEvent::BootstrapStatus {
        message: "resolving_version".to_string(),
    });
    steps.push(BootstrapStep::VersionResolution);
    let version_guid = runtime.resolve_version(context.mode, context.force_upgrade)?;

    record_event(DomainEvent::BootstrapStatus {
        message: "syncing_packages".to_string(),
    });
    steps.push(BootstrapStep::DownloadPackages);
    runtime.sync_packages(&version_guid)?;

    record_event(DomainEvent::BootstrapStatus {
        message: "applying_modifications".to_string(),
    });
    steps.push(BootstrapStep::ApplyModifications);
    runtime.apply_modifications(&version_guid)?;

    record_event(DomainEvent::BootstrapStatus {
        message: "registering_state".to_string(),
    });
    steps.push(BootstrapStep::RegisterSystemState);
    runtime.register_system_state(context.mode, &version_guid)?;

    if !context.no_launch {
        record_event(DomainEvent::BootstrapStatus {
            message: "launching_client".to_string(),
        });
        steps.push(BootstrapStep::LaunchClient);
        let pid = runtime.launch_client(context.mode, &context.launch_args)?;
        record_event(DomainEvent::BootstrapStatus {
            message: format!("launched_client_pid={pid}"),
        });
    } else if !context.quiet {
        record_event(DomainEvent::PromptRequired {
            kind: PromptKind::ConfirmLaunch,
            message: "launch_skipped_due_to_no_launch_flag".to_string(),
        });
    }

    steps.push(BootstrapStep::Completed);
    record_event(DomainEvent::Progress {
        current: 1,
        total: 1,
    });

    Ok(BootstrapReport { steps, events })
}
