// Tauri command bindings for Ruststrap
// These map to your Rust backend commands

import { invoke } from "@tauri-apps/api/core";

interface Settings {
  [key: string]: unknown;
}

interface RuntimeStatus {
  installed: boolean;
  uninstall_key_present: boolean;
  owns_player_protocol: boolean;
  owns_studio_protocol: boolean;
  install_required: boolean;
  relaunched?: boolean;
  [key: string]: unknown;
}

interface InstallResult {
  relaunched?: boolean;
  [key: string]: unknown;
}

interface RegionSelectorStatus {
  cookie_state: string;
  has_valid_cookie: boolean;
}

interface RegionDatacenters {
  regions: string[];
  datacenter_map: Record<string, string>;
}

interface RegionGameSearchEntry {
  universe_id: number;
  root_place_id: number;
  name: string;
  player_count?: number | null;
  thumbnail_url?: string | null;
}

interface RegionServerPage {
  data: Array<{
    job_id: string;
    playing: number;
    max_players: number;
    ping?: number | null;
    fps?: number | null;
    data_center_id?: number | null;
    region: string;
    uptime?: string | null;
  }>;
  next_cursor?: string | null;
}

interface StartupLaunch {
  mode: string;
  rawArgs?: string | null;
}

export const commands = {
  // setttings
  async getSettings(): Promise<Settings> {
    return invoke<Settings>("get_settings");
  },

  async saveSettings(settings: Settings): Promise<void> {
    // save_settings expects a json string payload named settingsJson.
    return invoke("save_settings", { settingsJson: JSON.stringify(settings) });
  },

  // ffs
  async getFastFlags(): Promise<Record<string, unknown>> {
    return invoke<Record<string, unknown>>("get_fast_flags");
  },

  async saveFastFlags(flags: Record<string, unknown>): Promise<void> {
    // save_fast_flags expects a json string payload named flagsJson.
    return invoke("save_fast_flags", { flagsJson: JSON.stringify(flags) });
  },

  // runtime
  async ensureRuntimeReady(): Promise<RuntimeStatus> {
    return invoke<RuntimeStatus>("ensure_runtime_ready");
  },

  async getRuntimeStatus(): Promise<RuntimeStatus> {
    return invoke<RuntimeStatus>("get_runtime_status");
  },

  // Installation
  async doFullInstall(desktop: boolean, startMenu: boolean, importOld: boolean): Promise<InstallResult> {
    return invoke<InstallResult>("do_full_install", {
      createDesktopShortcut: desktop,
      createStartMenuShortcut: startMenu,
      importFromRuststrap: importOld,
    });
  },

  // Launching
  async launchPlayer(rawArgs?: string): Promise<void> {
    return invoke("launch_player", { rawArgs });
  },

  async launchStudio(rawArgs?: string): Promise<void> {
    return invoke("launch_studio", { rawArgs });
  },

  async takeStartupLaunch(): Promise<StartupLaunch | null> {
    return invoke<StartupLaunch | null>("take_startup_launch");
  },

  // Modifications
  async applyModifications(): Promise<void> {
    return invoke("apply_modifications");
  },

  // Window controls
  async winMinimize(): Promise<void> {
    return invoke("win_minimize");
  },

  async winMaximize(): Promise<void> {
    return invoke("win_maximize");
  },

  async winClose(): Promise<void> {
    return invoke("win_close");
  },

  // Settings folder
  async openSettings(): Promise<void> {
    return invoke("open_settings");
  },

  // Region Selector
  async regionSelectorStatus(): Promise<RegionSelectorStatus> {
    return invoke<RegionSelectorStatus>("region_selector_status");
  },

  async regionSelectorDatacenters(): Promise<RegionDatacenters> {
    return invoke<RegionDatacenters>("region_selector_datacenters");
  },

  async regionSelectorSearchGames(query: string): Promise<RegionGameSearchEntry[]> {
    return invoke<RegionGameSearchEntry[]>("region_selector_search_games", { query });
  },

  async regionSelectorServers(
    placeId: number,
    cursor?: string,
    sortOrder?: number,
    selectedRegion?: string
  ): Promise<RegionServerPage> {
    return invoke<RegionServerPage>("region_selector_servers", {
      placeId,
      cursor,
      sortOrder,
      selectedRegion,
    });
  },

  async regionSelectorJoin(placeId: number, jobId: string): Promise<void> {
    return invoke("region_selector_join", { placeId, jobId });
  },
};
