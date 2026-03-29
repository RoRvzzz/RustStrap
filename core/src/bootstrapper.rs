/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use crate::ParsedLaunchSettings;
use crate::Result;
use crate::{run_bootstrap_flow, BootstrapContext, BootstrapReport, BootstrapRuntime, LaunchMode};

pub fn context_from_launch_settings(settings: &ParsedLaunchSettings) -> BootstrapContext {
    BootstrapContext {
        mode: settings.roblox_launch_mode,
        launch_args: settings.roblox_launch_args.clone(),
        quiet: settings.quiet_flag.active,
        force_upgrade: settings.force_flag.active,
        no_launch: settings.no_launch_flag.active,
    }
}

pub fn execute_bootstrap(
    runtime: &dyn BootstrapRuntime,
    settings: &ParsedLaunchSettings,
) -> Result<BootstrapReport> {
    runtime.prepare_launch_context(
        settings.roblox_launch_mode,
        &settings.roblox_launch_args,
        settings.channel_flag.data.as_deref(),
    )?;

    let mut context = context_from_launch_settings(settings);
    if context.mode == LaunchMode::Unknown {
        // this is for any unknown launch uri
        context.mode = LaunchMode::Player;
    }
    run_bootstrap_flow(runtime, &context)
}
