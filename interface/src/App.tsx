import React, { useEffect, useState } from "react";
import { commands } from "./commands";
import { 
  Puzzle, 
  Rocket, 
  Globe, 
  Gamepad2, 
  Settings2, 
  Palette, 
  Pin, 
  Info, 
  Menu, 
  MessageSquare, 
  Blocks, 
  BookOpen, 
  MapPin,
  ChevronDown,
  Plus,
  Trash2,
  ExternalLink
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";

interface Settings {
  AllowCookieAccess: boolean;
  BootstrapperStyle: number;
  BootstrapperIcon: number;
  BootstrapperTitle: string;
  BootstrapperIconCustomLocation: string;
  RobloxIcon: number;
  RobloxTitle: string;
  RobloxIconCustomLocation: string;
  Theme: number;
  DeveloperMode: boolean;
  ForceLocalData: boolean;
  CheckForUpdates: boolean;
  MultiInstanceLaunching: boolean;
  ConfirmLaunches: boolean;
  Locale: string;
  ForceRobloxLanguage: boolean;
  UseFastFlagManager: boolean;
  WPFSoftwareRender: boolean;
  EnableAnalytics: boolean;
  UpdateRoblox: boolean;
  StaticDirectory: boolean;
  Channel: string;
  ChannelChangeMode: number;
  ChannelHash: string;
  BackgroundUpdatesEnabled: boolean;
  EnableBetterMatchmaking: boolean;
  EnableBetterMatchmakingRandomization: boolean;
  CleanerOptions: number;
  CleanerDirectories: string[];
  FakeBorderlessFullscreen: boolean;
  EnableActivityTracking: boolean;
  UseDiscordRichPresence: boolean;
  HideRPCButtons: boolean;
  ShowAccountOnRichPresence: boolean;
  ShowServerDetails: boolean;
  CustomIntegrations: CustomIntegration[];
  UseDisableAppPatch: boolean;
  AutoRejoin: boolean;
  PlaytimeCounter: boolean;
  ShowServerUptime: boolean;
  ShowGameHistoryMenu: boolean;
  EnableCustomStatusDisplay: boolean;
  ShowUsingRuststrapRPC: boolean;
  StudioRPC: boolean;
  StudioThumbnailChanging: boolean;
  StudioEditingInfo: boolean;
  StudioWorkspaceInfo: boolean;
  StudioShowTesting: boolean;
  StudioGameButton: boolean;
  DisableRobloxRecording: boolean;
  DisableRobloxScreenshots: boolean;
  AutoCloseCrashHandler: boolean;
  Error773Fix: boolean;
  MultibloxInstanceCount: number;
  MultibloxDelayMs: number;
  SelectedProcessPriority: number;
  SelectedRegion: string;
  extra?: Record<string, unknown>;
  [key: string]: unknown;
}

interface CustomIntegration {
  Name: string;
  Location: string;
  LaunchArgs: string;
  Delay: number;
  AutoClose: boolean;
  PreLaunch: boolean;
}

interface RuntimeStatus {
  installed: boolean;
  uninstall_key_present: boolean;
  owns_player_protocol: boolean;
  owns_studio_protocol: boolean;
  install_required: boolean;
  running_exe_path?: string | null;
  running_exe_matches_expected?: boolean;
  running_binary_matches_expected?: boolean;
  runtime_reconcile_required?: boolean;
  relaunched?: boolean;
  expected_exe_path?: string;
  expected_player_command?: string;
  expected_studio_command?: string;
  actual_roblox_command?: string | null;
  actual_roblox_player_command?: string | null;
  actual_roblox_studio_command?: string | null;
  actual_roblox_studio_auth_command?: string | null;
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

interface RegionServerEntry {
  job_id: string;
  playing: number;
  max_players: number;
  ping?: number | null;
  fps?: number | null;
  data_center_id?: number | null;
  region: string;
  uptime?: string | null;
}

interface RegionServerPage {
  data: RegionServerEntry[];
  next_cursor?: string | null;
}

interface StartupLaunch {
  mode: string;
  rawArgs?: string | null;
}

const BOOTSTRAP_PHASE_TOTAL = 6;

function unwrapEnvelopePayload(payload: unknown): Record<string, unknown> {
  if (payload && typeof payload === "object" && "payload" in payload) {
    return (payload as { payload: Record<string, unknown> }).payload;
  }
  return payload as Record<string, unknown>;
}

function mapBootstrapDetail(detail: string): { label: string; current: number; total: number } | null {
  const phases: Record<string, { label: string; current: number }> = {
    checking_connectivity: { label: "Checking connectivity...", current: 1 },
    resolving_version: { label: "Resolving Roblox version...", current: 2 },
    syncing_packages: { label: "Downloading packages...", current: 3 },
    applying_modifications: { label: "Applying modifications...", current: 4 },
    registering_state: { label: "Registering runtime state...", current: 5 },
    launching_client: { label: "Launching Roblox...", current: 6 },
  };

  if (detail.startsWith("launched_client_pid=")) {
    return { label: "Roblox launched", current: 6, total: BOOTSTRAP_PHASE_TOTAL };
  }

  const phase = phases[detail];
  if (phase) {
    return { ...phase, total: BOOTSTRAP_PHASE_TOTAL };
  }
  return null;
}

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [fastFlags, setFastFlags] = useState<Record<string, string>>({});
  const [tab, setTab] = useState("integrations");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Ready");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [view, setView] = useState("launcher");
  const [appVersion, setAppVersion] = useState("0.1.0");

  const [isBootstrapping, setIsBootstrapping] = useState(false);
  const [bootstrapStatus, setBootstrapStatus] = useState("");
  const [progress, setProgress] = useState<{ current: number; total: number } | null>(null);

  const startBootstrapOverlay = (label: string) => {
    setIsBootstrapping(true);
    setBootstrapStatus(label);
    setProgress({ current: 0, total: BOOTSTRAP_PHASE_TOTAL });
  };

  useEffect(() => { load(); }, []);
  
  useEffect(() => {
    getVersion()
      .then((version) => setAppVersion(version))
      .catch(() => setAppVersion("0.1.0"));
  }, []);

  useEffect(() => {
    const unsubs: (() => void)[] = [];
    
    listen<unknown>("bootstrap_status", (e) => {
      const payload = unwrapEnvelopePayload(e.payload);
      const detail = payload?.detail;
      if (typeof detail !== "string" || !detail.trim()) return;

      const mapped = mapBootstrapDetail(detail);
      setIsBootstrapping(true);
      if (mapped) {
        setBootstrapStatus(mapped.label);
        setProgress({ current: mapped.current, total: mapped.total });
      } else {
        setBootstrapStatus(detail.replace(/_/g, " "));
      }
    }).then(u => unsubs.push(u));

    listen<unknown>("progress", (e) => {
      const payload = unwrapEnvelopePayload(e.payload);
      const current = Number(payload?.current);
      const total = Number(payload?.total);
      if (!Number.isFinite(current) || !Number.isFinite(total) || total <= 0) return;

      if (total <= 1 && current >= total) {
        setProgress({ current: BOOTSTRAP_PHASE_TOTAL, total: BOOTSTRAP_PHASE_TOTAL });
        return;
      }
      setProgress({ current, total });
    }).then(u => unsubs.push(u));

    listen<unknown>("fatal_error", (e) => {
      const payload = unwrapEnvelopePayload(e.payload);
      const message = typeof payload?.message === "string" ? payload.message : "Unknown fatal error";
      setIsBootstrapping(true);
      setBootstrapStatus(`Launch failed: ${message}`);
      setStatus("error");
    }).then(u => unsubs.push(u));

    return () => { unsubs.forEach(u => u()); };
  }, []);

  async function load() {
    setBusy(true);
    let startupLaunch: StartupLaunch | null = null;
    try {
      const s = await commands.getSettings();
      setSettings(s as Settings);
      const f = await commands.getFastFlags();
      setFastFlags(f || {});
      const runtime = await commands.ensureRuntimeReady();
      setRuntimeStatus(runtime);
      if (runtime?.relaunched) {
        await commands.winClose();
        return;
      }
      if (runtime?.install_required) {
        setView("installer");
      }
      startupLaunch = (await commands.takeStartupLaunch()) as StartupLaunch | null;
      setStatus("Settings loaded");
    } catch (e: unknown) {
      try {
        const runtime = await commands.getRuntimeStatus();
        setRuntimeStatus(runtime);
        if (runtime?.install_required) {
          setView("installer");
        }
      } catch (_) { /* ignore */ }
      setStatus(`Load error: ${e}`);
    } finally { 
      setBusy(false); 
    }

    const mode = (startupLaunch?.mode || "").toLowerCase();
    if (mode === "player" || mode === "studio" || mode === "studio_auth") {
      startBootstrapOverlay(mode === "player" ? "Starting Roblox..." : "Starting Studio...");
      try {
        if (mode === "player") {
          await commands.launchPlayer(startupLaunch?.rawArgs || undefined);
        } else {
          await commands.launchStudio(startupLaunch?.rawArgs || undefined);
        }
        await commands.winClose();
      } catch (error: unknown) {
        setIsBootstrapping(false);
        setView("settings");
        setStatus(`Startup launch error: ${String(error)}`);
      }
    }
  }

  async function save() {
    if (!settings) return;
    setBusy(true);
    try {
      await commands.saveSettings(settings);
      await commands.saveFastFlags(fastFlags);
      try { await commands.applyModifications(); } catch (_) { /* ignore */ }
      setStatus("Saved & applied");
    } catch (e: unknown) {
      setStatus(`Save error: ${e}`);
    } finally { 
      setBusy(false); 
    }
  }

  async function saveAndLaunch() {
    if (!settings) return;
    setBusy(true);
    try {
      const runtime = await commands.ensureRuntimeReady();
      setRuntimeStatus(runtime);
      if (runtime?.relaunched) {
        await commands.winClose();
        return;
      }
      if (runtime?.install_required) {
        setView("installer");
        setStatus("Runtime setup is required");
        return;
      }

      await commands.saveSettings(settings);
      await commands.saveFastFlags(fastFlags);
      try { await commands.applyModifications(); } catch (_) { /* ignore */ }
      startBootstrapOverlay("Starting Roblox...");
      await commands.launchPlayer();
      await commands.winClose();
    } catch (e: unknown) {
      setView("installer");
      setBootstrapStatus(`Launch error: ${e}`);
      setStatus("error");
    } finally {
      setBusy(false);
    }
  }

  async function launchStudio() {
    setBusy(true);
    try {
      const runtime = await commands.ensureRuntimeReady();
      setRuntimeStatus(runtime);
      if (runtime?.relaunched) {
        await commands.winClose();
        return;
      }
      if (runtime?.install_required) {
        setView("installer");
        setStatus("Runtime setup is required");
        return;
      }
      await commands.launchStudio();
    } catch (e: unknown) {
      setView("installer");
      setStatus(`Studio launch error: ${e}`);
    } finally {
      setBusy(false);
    }
  }

  const set = <K extends keyof Settings>(k: K, v: Settings[K]) =>
    setSettings(prev => prev ? { ...prev, [k]: v } : prev);

  if (!settings) {
    return (
      <div className="app-shell">
        <div className="app-body" style={{ display: "flex", justifyContent: "center", alignItems: "center" }}>
          <div className="loading-spinner" />
        </div>
      </div>
    );
  }

  const tabs = [
    { id: "integrations", label: "Integrations", icon: <Puzzle size={18} /> },
    { id: "bootstrapper", label: "Bootstrapper", icon: <Rocket size={18} /> },
    { id: "region", label: "Region Selector", icon: <MapPin size={18} /> },
    { id: "deployment", label: "Deployment", icon: <Globe size={18} /> },
    { id: "mods", label: "Mods", icon: <Gamepad2 size={18} /> },
    { id: "fastflags", label: "Fast Flags", icon: <Settings2 size={18} /> },
    { id: "appearance", label: "Appearance", icon: <Palette size={18} /> },
    { id: "shortcuts", label: "Shortcuts", icon: <Pin size={18} /> },
    { id: "about", label: "About", icon: <Info size={18} /> },
  ];

  if (isBootstrapping) {
    return (
      <div className="bootstrap-overlay">
        <div className="titlebar" data-tauri-drag-region style={{ position: "absolute", top: 0, width: "100%", zIndex: 10 }}>
          <h1 className="titlebar-title" data-tauri-drag-region>{settings.BootstrapperTitle || "Ruststrap"}</h1>
          <div className="titlebar-controls">
            <button className="win-btn win-close" onClick={() => void commands.winClose()} title="Close">&#x2715;</button>
          </div>
        </div>
        <div style={{ textAlign: "center", maxWidth: 400, zIndex: 1 }}>
          <h2 style={{ fontSize: "1rem", fontWeight: 500, marginBottom: 16, color: "var(--text)" }}>{bootstrapStatus}</h2>
          <div className="bootstrap-progress-bar">
            <div
              className={`bootstrap-progress-fill${progress ? "" : " indeterminate"}`}
              style={
                progress && progress.total > 0
                  ? { width: `${Math.min(100, (progress.current / progress.total) * 100)}%` }
                  : undefined
              }
            />
          </div>
          {status.includes("error") && (
            <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 20, alignItems: "center" }}>
              <button className="btn-secondary" onClick={() => setIsBootstrapping(false)}>Back to Settings</button>
            </div>
          )}
        </div>
      </div>
    );
  }

  if (view === "launcher") {
    return (
      <LauncherView
        setView={setView}
        saveAndLaunch={saveAndLaunch}
        launchStudio={launchStudio}
        appVersion={appVersion}
      />
    );
  }

  if (view === "installer") {
    return <InstallerView setView={setView} settings={settings} set={set} />;
  }

  return (
    <div className="app-shell">
      <div className="titlebar" data-tauri-drag-region>
        <h1 className="titlebar-title" data-tauri-drag-region>Ruststrap Settings</h1>
        <div className="titlebar-controls">
          <button className="win-btn" onClick={() => void commands.winMinimize()} title="Minimize">&#x2500;</button>
          <button className="win-btn" onClick={() => void commands.winMaximize()} title="Maximize">&#x25A1;</button>
          <button className="win-btn win-close" onClick={() => void commands.winClose()} title="Close">&#x2715;</button>
        </div>
      </div>
      <div className={`app-body ${!sidebarOpen ? "sidebar-collapsed" : ""}`}>
        <aside className="sidebar">
          <div style={{ padding: "8px 12px" }}>
            <div 
              className="nav-item" 
              style={{ width: "fit-content", padding: 10, cursor: "pointer" }} 
              onClick={() => setSidebarOpen(!sidebarOpen)}
            >
              <Menu size={18} />
            </div>
          </div>
          <nav>
            {tabs.map((t) => (
              <div 
                key={t.id} 
                className={`nav-item${tab === t.id ? " active" : ""}`} 
                onClick={() => setTab(t.id)}
              >
                {t.icon}
                {sidebarOpen && <span>{t.label}</span>}
              </div>
            ))}
          </nav>
        </aside>
        <main className="main-content">
          {tab === "integrations" && <PageIntegrations s={settings} set={set} />}
          {tab === "bootstrapper" && <PageBootstrapper s={settings} set={set} />}
          {tab === "region" && <PageRegionSelector s={settings} set={set} />}
          {tab === "deployment" && <PageDeployment s={settings} set={set} />}
          {tab === "mods" && <PageMods s={settings} set={set} />}
          {tab === "fastflags" && <PageFastFlags flags={fastFlags} setFlags={setFastFlags} s={settings} set={set} />}
          {tab === "appearance" && <PageAppearance s={settings} set={set} />}
          {tab === "shortcuts" && <PageShortcuts />}
          {tab === "about" && <PageAbout />}
        </main>
      </div>
      <div className="footer-bar">
        <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{status}</span>
        <div className="footer-actions">
          <button className="btn-primary" disabled={busy} onClick={() => void saveAndLaunch()}>
            Save and Launch
          </button>
          <button className="btn-secondary" disabled={busy} onClick={() => void save()}>
            Save
          </button>
          <button className="btn-secondary" onClick={() => void commands.winClose()}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

/* ========================================
   LAUNCHER VIEW
   ======================================== */

interface LauncherViewProps {
  setView: (view: string) => void;
  saveAndLaunch: () => Promise<void>;
  launchStudio: () => Promise<void>;
  appVersion: string;
}

function LauncherView({ setView, saveAndLaunch, launchStudio, appVersion }: LauncherViewProps) {
  return (
    <div className="launcher-page">
      <div className="titlebar" data-tauri-drag-region style={{ position: "absolute", top: 0, width: "100%", zIndex: 10 }}>
        <h1 className="titlebar-title" data-tauri-drag-region></h1>
        <div className="titlebar-controls">
          <button className="win-btn win-close" onClick={() => void commands.winClose()} title="Close">&#x2715;</button>
        </div>
      </div>
      <div className="launcher-content" data-tauri-drag-region>
        <div className="launcher-left">
          <div className="launcher-brand">
            <div className="launcher-brand-icon">
              <img src="/icon.png" alt="Ruststrap" width={64} height={64} />
            </div>
            <div className="launcher-brand-text">
              <h2>Ruststrap</h2>
              <span>Version {appVersion}</span>
            </div>
          </div>
          <div className="launcher-links">
            <a href="#" onClick={(e) => { e.preventDefault(); setView("installer"); }}>
              <Info size={16} />
              <span>Installer Tool</span>
            </a>
            <a href="https://discord.gg/KdR9vpRcUN" target="_blank" rel="noopener noreferrer">
              <MessageSquare size={16} />
              <span>Join our Discord</span>
            </a>
          </div>
        </div>
        <div className="launcher-right">
          <button className="action-card primary" onClick={() => void saveAndLaunch()}>
            <Gamepad2 size={24} />
            <span>Launch Roblox</span>
          </button>
          <div className="action-row">
            <button className="action-card secondary" onClick={() => setView("settings")}>
              <Settings2 size={20} />
              <span>Settings</span>
            </button>
            <button className="action-card secondary" onClick={() => void launchStudio()}>
              <Blocks size={20} />
              <span>Studio</span>
            </button>
          </div>
          <div className="launcher-divider" />
          <button className="action-card tertiary" onClick={() => window.open("https://github.com/Ruststrap/Ruststrap/wiki")}>
            <BookOpen size={20} />
            <div className="action-card-text">
              <strong>Having an issue?</strong>
              <span>See the Wiki for help</span>
            </div>
            <ExternalLink size={14} style={{ marginLeft: "auto", opacity: 0.5 }} />
          </button>
        </div>
      </div>
    </div>
  );
}

/* ========================================
   INSTALLER VIEW
   ======================================== */

const SUPPORTED_LOCALES: [string, string][] = [
  ["nil", "System Default"],
  ["en", "English"],
  ["en-US", "English (United States)"],
  ["ar", "العربية"],
  ["bg", "Български"],
  ["bs", "Bosanski"],
  ["cs", "Čeština"],
  ["da", "Dansk"],
  ["de", "Deutsch"],
  ["es-ES", "Español"],
  ["fa", "فارسی"],
  ["fi", "Suomi"],
  ["fil", "Filipino"],
  ["fr", "Français"],
  ["hr", "Hrvatski"],
  ["hu", "Magyar"],
  ["id", "Bahasa Indonesia"],
  ["it", "Italiano"],
  ["ja", "日本語"],
  ["ko", "한국어"],
  ["lv", "Latviešu"],
  ["lt", "Lietuvių"],
  ["ms", "Malay"],
  ["nl", "Nederlands"],
  ["pl", "Polski"],
  ["pt-BR", "Português (Brasil)"],
  ["ro", "Română"],
  ["ru", "Русский"],
  ["sv-SE", "Svenska"],
  ["th", "ภาษาไทย"],
  ["tr", "Türkçe"],
  ["uk", "Українська"],
  ["vi", "Tiếng Việt"],
  ["zh-CN", "中文 (简体)"],
  ["zh-HK", "中文 (香港)"],
  ["zh-TW", "中文 (繁體)"],
];

interface InstallerViewProps {
  setView: (view: string) => void;
  settings: Settings;
  set: <K extends keyof Settings>(k: K, v: Settings[K]) => void;
}

function InstallerView({ setView, settings, set }: InstallerViewProps) {
  const [step, setStep] = useState(0);
  const [selectedLocale, setSelectedLocale] = useState(settings?.Locale || "nil");
  const [installLoc, setInstallLoc] = useState("");
  const [desktop, setDesktop] = useState(true);
  const [startMenu, setStartMenu] = useState(true);
  const [importOld, setImportOld] = useState(false);
  const [importSource, setImportSource] = useState("Ruststrap");
  const [error, setError] = useState("");

  const stepLabels = ["Welcome", "Install", "Finish"];

  const goNext = () => {
    if (step === 0) {
      set("Locale", selectedLocale);
      setStep(1);
    } else if (step === 1) {
      setStep(2);
    } else {
      doInstall();
    }
  };

  const goBack = () => {
    if (step > 0) setStep(step - 1);
  };

  const doInstall = async () => {
    try {
      setError("");
      const result = await commands.doFullInstall(desktop, startMenu, importOld);
      if (result?.relaunched) {
        await commands.winClose();
        return;
      }
      const runtime = await commands.ensureRuntimeReady();
      if (runtime?.install_required) {
        setError("Install completed but runtime ownership repair is still required.");
        return;
      }
      setView("launcher");
    } catch (e: unknown) {
      setError("Install error: " + e);
    }
  };

  return (
    <div className="installer-page">
      <div className="titlebar" data-tauri-drag-region>
        <h1 className="titlebar-title" data-tauri-drag-region>Ruststrap Installer</h1>
        <div className="titlebar-controls">
          <button className="win-btn win-close" onClick={() => void commands.winClose()} title="Close">&#x2715;</button>
        </div>
      </div>

      <div className="installer-steps">
        {stepLabels.map((label, i) => (
          <React.Fragment key={label}>
            <div className={`installer-step ${i === step ? "active" : i < step ? "done" : ""}`}>
              <div className="installer-step-num">{i < step ? "✓" : i + 1}</div>
              <span>{label}</span>
            </div>
            {i < stepLabels.length - 1 && <div className="installer-step-line" />}
          </React.Fragment>
        ))}
      </div>

      <div className="installer-content">
        {step === 0 && (
          <>
            <h2>Welcome to Ruststrap</h2>
            <p style={{ marginBottom: 20 }}>
              Ruststrap is a high-performance bootstrapper for Roblox. Select your preferred language below, then click Next to continue.
            </p>
            <h2 style={{ fontSize: 15, marginBottom: 10 }}>Language</h2>
            <div className="language-grid">
              {SUPPORTED_LOCALES.map(([code, name]) => (
                <button
                  key={code}
                  className={`language-option ${selectedLocale === code ? "selected" : ""}`}
                  onClick={() => setSelectedLocale(code)}
                >
                  {name}
                </button>
              ))}
            </div>
          </>
        )}

        {step === 1 && (
          <>
            <h2>Install Location</h2>
            <p style={{ marginBottom: 12 }}>Choose where Ruststrap should be installed.</p>
            <CardGroup>
              <div style={{ display: "flex", gap: 8, padding: 14 }}>
                <input 
                  type="text" 
                  value={installLoc} 
                  onChange={e => setInstallLoc(e.target.value)} 
                  style={{ flex: 1 }} 
                  placeholder="C:\Users\...\AppData\Local\Ruststrap" 
                />
                <button className="btn-secondary" onClick={() => setInstallLoc("")}>Reset</button>
              </div>
            </CardGroup>

            <h2 style={{ marginTop: 24 }}>Shortcuts</h2>
            <p style={{ marginBottom: 12 }}>Choose which shortcuts should be created.</p>
            <CardGroup>
              <Opt header="Desktop Shortcut" desc="Create a shortcut on your desktop.">
                <Toggle checked={desktop} onChange={setDesktop} />
              </Opt>
              <Opt header="Start Menu Shortcut" desc="Create a shortcut in the Start menu.">
                <Toggle checked={startMenu} onChange={setStartMenu} />
              </Opt>
            </CardGroup>
          </>
        )}

        {step === 2 && (
          <>
            <h2>Import Settings</h2>
            <p style={{ marginBottom: 12 }}>Optionally import settings from an existing installation.</p>
            <CardGroup>
              <Opt header="Import settings from legacy" desc="Copy settings, mods, and themes from a previous installation.">
                <Toggle checked={importOld} onChange={setImportOld} />
              </Opt>
            </CardGroup>
            {importOld && (
              <div style={{ marginTop: 16 }}>
                <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 8 }}>Import from:</p>
                <div className="import-selector">
                  {["Ruststrap", "Fishstrap", "Froststrap"].map(src => (
                    <button 
                      key={src} 
                      className={`import-option ${importSource === src ? "selected" : ""}`} 
                      onClick={() => setImportSource(src)}
                    >
                      {src}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {error && <WarningBanner>{error}</WarningBanner>}

            <div style={{ marginTop: 24, padding: 16, background: "var(--bg-input)", borderRadius: 8, border: "1px solid var(--border)" }}>
              <p style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.6 }}>
                Click <strong style={{ color: "var(--text)" }}>Install</strong> to complete the setup. Ruststrap will register itself as the default Roblox bootstrapper.
              </p>
            </div>
          </>
        )}
      </div>

      <div className="installer-footer">
        <button className="btn-secondary" onClick={() => step === 0 ? setView("launcher") : goBack()}>
          {step === 0 ? "Cancel" : "Back"}
        </button>
        <div className="installer-footer-actions">
          <button className="btn-primary" onClick={goNext}>
            {step === 2 ? "Install" : "Next"}
          </button>
          <button className="btn-secondary" onClick={() => void commands.winClose()}>Close</button>
        </div>
      </div>
    </div>
  );
}

/* ========================================
   SHARED COMPONENTS
   ======================================== */

type SettingsProps = { 
  s: Settings; 
  set: <K extends keyof Settings>(k: K, v: Settings[K]) => void;
};

function CardGroup({ children }: { children: React.ReactNode }) {
  return <div className="card-group">{children}</div>;
}

function Section({ title, desc, children }: { title: string; desc?: string; children?: React.ReactNode }) {
  return (
    <div className="section-container">
      {title && <div className="group-header">{title}</div>}
      {desc && <p className="group-desc">{desc}</p>}
      {children && <div className="card-group">{children}</div>}
    </div>
  );
}

function Opt({ header, desc, children, disabled }: { header: string; desc?: string; children: React.ReactNode; disabled?: boolean }) {
  return (
    <div className="control-row" style={disabled ? { opacity: 0.45, pointerEvents: "none" } : undefined}>
      <div className="control-info">
        <span className="control-header">{header}</span>
        {desc && <span className="control-desc">{desc}</span>}
      </div>
      <div className="control-widget">{children}</div>
    </div>
  );
}

function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <label className={`toggle${checked ? " on" : ""}`} style={disabled ? { opacity: 0.5, pointerEvents: "none" } : undefined}>
      <input type="checkbox" checked={checked} onChange={e => onChange(e.target.checked)} disabled={disabled} />
      <span className="toggle-track"><span className="toggle-thumb" /></span>
    </label>
  );
}

function WarningBanner({ children }: { children: React.ReactNode }) {
  return <div className="warning-banner">{children}</div>;
}

function Expander({ title, desc, children }: { title: string; desc?: string; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="card-group" style={{ marginBottom: 4 }}>
      <div className="expander-header" onClick={() => setOpen(!open)}>
        <div className="control-info">
          <span className="control-header">{title}</span>
          {desc && <span className="control-desc">{desc}</span>}
        </div>
        <ChevronDown size={14} className={`expander-icon ${open ? "open" : ""}`} />
      </div>
      {open && <div className="expander-content">{children}</div>}
    </div>
  );
}

/* ========================================
   PAGE: INTEGRATIONS
   ======================================== */

function PageIntegrations({ s, set }: SettingsProps) {
  const [selIdx, setSelIdx] = useState(-1);
  const integrations: CustomIntegration[] = (s.CustomIntegrations || []);

  const updateIntegration = (idx: number, patch: Partial<CustomIntegration>) => {
    const next = [...integrations];
    next[idx] = { ...next[idx], ...patch };
    set("CustomIntegrations", next);
  };

  const addIntegration = () => {
    const next = [...integrations, { Name: "New Program", Location: "", LaunchArgs: "", Delay: 200, AutoClose: false, PreLaunch: false }];
    set("CustomIntegrations", next);
    setSelIdx(next.length - 1);
  };

  const delIntegration = () => {
    if (selIdx < 0) return;
    const next = integrations.filter((_, i) => i !== selIdx);
    set("CustomIntegrations", next);
    setSelIdx(-1);
  };

  const sel = selIdx >= 0 && selIdx < integrations.length ? integrations[selIdx] : null;

  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Integrations</h2>
        <p>Configure additional functionality to go alongside Roblox.</p>
      </hgroup>

      <Section title="Activity Tracking">
        <Opt header="Enable activity tracking" desc="Allows Ruststrap to detect what Roblox game you're playing.">
          <Toggle checked={s.EnableActivityTracking} onChange={v => set("EnableActivityTracking", v)} />
        </Opt>
        <Opt header="Query server details" desc="See server location via rovalra.com." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.ShowServerDetails} onChange={v => set("ShowServerDetails", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Show server uptime" desc="Display how long the current server has been running." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.ShowServerUptime} onChange={v => set("ShowServerUptime", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Auto-rejoin on disconnect" desc="Automatically rejoin when disconnected due to inactivity." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.AutoRejoin} onChange={v => set("AutoRejoin", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Playtime counter" desc="Tracks total and per-session playtime." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.PlaytimeCounter} onChange={v => set("PlaytimeCounter", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Show game history menu" desc="Show recently played games in system tray." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.ShowGameHistoryMenu} onChange={v => set("ShowGameHistoryMenu", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Don't exit to desktop app" desc="Fully close Roblox when leaving a game." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.UseDisableAppPatch} onChange={v => set("UseDisableAppPatch", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
      </Section>

      <Section title="Discord Rich Presence" desc="Requires activity tracking and the Discord desktop app.">
        <Opt header="Show game activity" desc="Display what you're playing on your Discord profile." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.UseDiscordRichPresence} onChange={v => set("UseDiscordRichPresence", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Allow activity joining" desc="Let others join your game through Discord." disabled={!s.UseDiscordRichPresence || !s.EnableActivityTracking}>
          <Toggle checked={!s.HideRPCButtons} onChange={v => set("HideRPCButtons", !v)} disabled={!s.UseDiscordRichPresence} />
        </Opt>
        <Opt header="Custom status display" desc="Replace 'Playing Roblox' with the game name." disabled={!s.UseDiscordRichPresence || !s.EnableActivityTracking}>
          <Toggle checked={s.EnableCustomStatusDisplay} onChange={v => set("EnableCustomStatusDisplay", v)} disabled={!s.UseDiscordRichPresence} />
        </Opt>
        <Opt header="Show Roblox account" desc="Display your Roblox account on Discord." disabled={!s.UseDiscordRichPresence || !s.EnableActivityTracking}>
          <Toggle checked={s.ShowAccountOnRichPresence} onChange={v => set("ShowAccountOnRichPresence", v)} disabled={!s.UseDiscordRichPresence} />
        </Opt>
        <Opt header="Show using Ruststrap" desc="Display that you're using Ruststrap." disabled={!s.UseDiscordRichPresence || !s.EnableActivityTracking}>
          <Toggle checked={s.ShowUsingRuststrapRPC} onChange={v => set("ShowUsingRuststrapRPC", v)} disabled={!s.UseDiscordRichPresence} />
        </Opt>
      </Section>

      <Section title="Studio Rich Presence" desc="Show what you're working on in Roblox Studio.">
        <Opt header="Enable Studio RPC" desc="Show Roblox Studio activity on Discord." disabled={!s.EnableActivityTracking}>
          <Toggle checked={s.StudioRPC} onChange={v => set("StudioRPC", v)} disabled={!s.EnableActivityTracking} />
        </Opt>
        <Opt header="Show script thumbnail" desc="Change thumbnail based on open script." disabled={!s.StudioRPC}>
          <Toggle checked={s.StudioThumbnailChanging} onChange={v => set("StudioThumbnailChanging", v)} disabled={!s.StudioRPC} />
        </Opt>
        <Opt header="Show editing info" desc="Show script type, name, and line count." disabled={!s.StudioRPC}>
          <Toggle checked={s.StudioEditingInfo} onChange={v => set("StudioEditingInfo", v)} disabled={!s.StudioRPC} />
        </Opt>
        <Opt header="Show workspace info" desc="Display current workspace information." disabled={!s.StudioRPC}>
          <Toggle checked={s.StudioWorkspaceInfo} onChange={v => set("StudioWorkspaceInfo", v)} disabled={!s.StudioRPC} />
        </Opt>
        <Opt header="Show testing status" desc="Show when testing your game in Studio." disabled={!s.StudioRPC}>
          <Toggle checked={s.StudioShowTesting} onChange={v => set("StudioShowTesting", v)} disabled={!s.StudioRPC} />
        </Opt>
        <Opt header="Show game button" desc="Add button to visit game page from Discord." disabled={!s.StudioRPC}>
          <Toggle checked={s.StudioGameButton} onChange={v => set("StudioGameButton", v)} disabled={!s.StudioRPC} />
        </Opt>
      </Section>

      <Section title="Roblox Media" desc="Block Roblox's built-in screenshot and recording features.">
        <Opt header="Disable Roblox recording" desc="Block video recordings to your Videos folder.">
          <Toggle checked={s.DisableRobloxRecording} onChange={v => set("DisableRobloxRecording", v)} />
        </Opt>
        <Opt header="Disable Roblox screenshots" desc="Block screenshots to your Pictures folder.">
          <Toggle checked={s.DisableRobloxScreenshots} onChange={v => set("DisableRobloxScreenshots", v)} />
        </Opt>
      </Section>

      <WarningBanner>Multi-Instancing is prone to breaking. We will not provide support for issues with Multi-Instancing.</WarningBanner>
      
      <Section title="Miscellaneous">
        <Opt header="Allow multi-instance launching" desc="Run more than one Roblox client simultaneously.">
          <Toggle checked={s.MultiInstanceLaunching} onChange={v => set("MultiInstanceLaunching", v)} />
        </Opt>
      </Section>

      <Section title="Custom Integrations" desc="Have other programs launch with Roblox automatically." />
      <div className="integrations-panel">
        <div className="integrations-list">
          <div className="integrations-items">
            {integrations.map((ci, i) => (
              <div 
                key={i} 
                className={`integration-row${selIdx === i ? " selected" : ""}`} 
                onClick={() => setSelIdx(i)}
              >
                {ci.Name || "(unnamed)"}
              </div>
            ))}
            {integrations.length === 0 && (
              <div style={{ padding: 14, opacity: 0.4, textAlign: "center", fontSize: 13 }}>No integrations</div>
            )}
          </div>
          <div className="integrations-actions">
            <button className="btn-secondary btn-sm" onClick={addIntegration}>
              <Plus size={14} /> New
            </button>
            <button className="btn-secondary btn-sm btn-danger" disabled={selIdx < 0} onClick={delIntegration}>
              <Trash2 size={14} /> Delete
            </button>
          </div>
        </div>
        {sel ? (
          <div className="integrations-detail">
            <label className="field-label">Name</label>
            <input type="text" value={sel.Name} onChange={e => updateIntegration(selIdx, { Name: e.target.value })} />
            <label className="field-label">Application Location</label>
            <input type="text" placeholder="C:\Windows\System32\cmd.exe" value={sel.Location} onChange={e => updateIntegration(selIdx, { Location: e.target.value })} />
            <div style={{ display: "flex", gap: 12 }}>
              <div style={{ flex: 1 }}>
                <label className="field-label">Launch Delay (ms)</label>
                <input type="number" value={sel.Delay} onChange={e => updateIntegration(selIdx, { Delay: Number(e.target.value) })} />
              </div>
              <div style={{ flex: 2 }}>
                <label className="field-label">Launch Args</label>
                <input type="text" placeholder="/k echo hello" value={sel.LaunchArgs} onChange={e => updateIntegration(selIdx, { LaunchArgs: e.target.value })} />
              </div>
            </div>
            <div style={{ display: "flex", gap: 16, marginTop: 8 }}>
              <label className="inline-check">
                <input type="checkbox" checked={sel.AutoClose} onChange={e => updateIntegration(selIdx, { AutoClose: e.target.checked })} />
                Auto Close
              </label>
              <label className="inline-check">
                <input type="checkbox" checked={sel.PreLaunch} onChange={e => updateIntegration(selIdx, { PreLaunch: e.target.checked })} />
                Pre Roblox Launch
              </label>
            </div>
          </div>
        ) : (
          <div className="integrations-detail" style={{ display: "flex", alignItems: "center", justifyContent: "center", opacity: 0.4, fontSize: 13 }}>
            Select or add an integration
          </div>
        )}
      </div>
    </div>
  );
}

/* ========================================
   PAGE: BOOTSTRAPPER
   ======================================== */

function PageBootstrapper({ s, set }: SettingsProps) {
  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Bootstrapper</h2>
        <p>Configure what Ruststrap should do when launching Roblox.</p>
      </hgroup>

      <CardGroup>
        <Opt header="Confirm when launching another instance" desc="Prevent accidentally closing your existing game.">
          <Toggle checked={s.ConfirmLaunches} onChange={v => set("ConfirmLaunches", v)} />
        </Opt>
        <Opt header="Allow unsupported Roblox languages" desc="Only applies to games launched from the Roblox website.">
          <Toggle checked={s.ForceRobloxLanguage} onChange={v => set("ForceRobloxLanguage", v)} />
        </Opt>
        <Opt header="Background updates" desc="Update Roblox in the background instead of waiting.">
          <Toggle checked={s.BackgroundUpdatesEnabled} onChange={v => set("BackgroundUpdatesEnabled", v)} />
        </Opt>
        <Opt header="Auto-close crash handler" desc="Automatically close the Roblox Crash Handler process.">
          <Toggle checked={s.AutoCloseCrashHandler} onChange={v => set("AutoCloseCrashHandler", v)} />
        </Opt>
      </CardGroup>

      <Section title="Process Priority" desc="Change the Roblox process priority to improve performance.">
        <Opt header="Roblox process priority" desc="Higher priorities may improve responsiveness.">
          <select value={s.SelectedProcessPriority} onChange={e => set("SelectedProcessPriority", Number(e.target.value))}>
            <option value={0}>Low</option>
            <option value={1}>Below Normal</option>
            <option value={2}>Normal</option>
            <option value={3}>Above Normal</option>
            <option value={4}>High</option>
            <option value={5}>Real Time</option>
          </select>
        </Opt>
      </Section>

      <Section title="Multi-Instance" desc="Configure multi-instance launching behavior.">
        <Opt header="Error 773 Fix" desc="Fix Error 773 when using multi-instance." disabled={!s.MultiInstanceLaunching}>
          <Toggle checked={s.Error773Fix} onChange={v => set("Error773Fix", v)} disabled={!s.MultiInstanceLaunching} />
        </Opt>
        <Opt header="Instance count" desc="Number of Roblox instances to launch." disabled={!s.MultiInstanceLaunching}>
          <input type="number" min={2} max={10} value={s.MultibloxInstanceCount ?? 2} onChange={e => set("MultibloxInstanceCount", Number(e.target.value))} style={{ width: 80 }} disabled={!s.MultiInstanceLaunching} />
        </Opt>
        <Opt header="Instance delay (ms)" desc="Delay between launching each instance." disabled={!s.MultiInstanceLaunching}>
          <input type="number" min={500} max={10000} step={100} value={s.MultibloxDelayMs ?? 1500} onChange={e => set("MultibloxDelayMs", Number(e.target.value))} style={{ width: 100 }} disabled={!s.MultiInstanceLaunching} />
        </Opt>
      </Section>

      <Expander title="Ruststrap Cleaner" desc="Remove old data to save space.">
        <Opt header="Delete files older than" desc="Files older than this will be deleted.">
          <select value={s.CleanerOptions} onChange={e => set("CleanerOptions", Number(e.target.value))}>
            <option value={0}>Never</option>
            <option value={1}>1 Day</option>
            <option value={2}>1 Week</option>
            <option value={3}>1 Month</option>
            <option value={4}>2 Months</option>
          </select>
        </Opt>
        <Opt header="Cache" desc="Old downloads will be deleted.">
          <Toggle checked={(s.CleanerDirectories || []).includes("cache")} onChange={v => {
            const dirs = [...(s.CleanerDirectories || [])];
            if (v && !dirs.includes("cache")) dirs.push("cache");
            if (!v) { const i = dirs.indexOf("cache"); if (i >= 0) dirs.splice(i, 1); }
            set("CleanerDirectories", dirs);
          }} />
        </Opt>
        <Opt header="Logs" desc="Old log files will be deleted.">
          <Toggle checked={(s.CleanerDirectories || []).includes("logs")} onChange={v => {
            const dirs = [...(s.CleanerDirectories || [])];
            if (v && !dirs.includes("logs")) dirs.push("logs");
            if (!v) { const i = dirs.indexOf("logs"); if (i >= 0) dirs.splice(i, 1); }
            set("CleanerDirectories", dirs);
          }} />
        </Opt>
        <Opt header="Ruststrap logs" desc="Ruststrap logs will be deleted.">
          <Toggle checked={(s.CleanerDirectories || []).includes("ruststrap_logs")} onChange={v => {
            const dirs = [...(s.CleanerDirectories || [])];
            if (v && !dirs.includes("ruststrap_logs")) dirs.push("ruststrap_logs");
            if (!v) { const i = dirs.indexOf("ruststrap_logs"); if (i >= 0) dirs.splice(i, 1); }
            set("CleanerDirectories", dirs);
          }} />
        </Opt>
      </Expander>

      <Section title="Experimental" desc="These settings may or may not work as intended.">
        <Opt header="Allow Ruststrap cookie access" desc="Provide access to Roblox APIs using your auth cookie.">
          <Toggle checked={s.AllowCookieAccess} onChange={v => set("AllowCookieAccess", v)} />
        </Opt>
        <Opt header="Enable BetterMatchmaking" desc="Let Ruststrap decide which servers you join.">
          <Toggle checked={s.EnableBetterMatchmaking} onChange={v => set("EnableBetterMatchmaking", v)} />
        </Opt>
        <Opt header="Randomize BetterMatchmaking" desc="Randomize the chosen server from optimal servers.">
          <Toggle checked={s.EnableBetterMatchmakingRandomization} onChange={v => set("EnableBetterMatchmakingRandomization", v)} />
        </Opt>
        <Opt header="Borderless Fullscreen for Vulkan" desc="Fake borderless fullscreen while using Vulkan.">
          <Toggle checked={s.FakeBorderlessFullscreen} onChange={v => set("FakeBorderlessFullscreen", v)} />
        </Opt>
      </Section>
    </div>
  );
}

/* ========================================
   PAGE: REGION SELECTOR
   ======================================== */

function PageRegionSelector({ s, set }: SettingsProps) {
  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Region Selector</h2>
        <p>Search games, browse public servers by region, and join directly.</p>
      </hgroup>
      <RegionBrowser s={s} set={set} />
    </div>
  );
}

function RegionBrowser({ s, set }: SettingsProps) {
  const [status, setStatus] = useState<RegionSelectorStatus | null>(null);
  const [datacenters, setDatacenters] = useState<RegionDatacenters | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<RegionGameSearchEntry[]>([]);
  const [placeIdInput, setPlaceIdInput] = useState("");
  const [sortOrder, setSortOrder] = useState(2);
  const [loadingServers, setLoadingServers] = useState(false);
  const [servers, setServers] = useState<RegionServerEntry[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const init = async () => {
      try {
        const [selectorStatus, selectorDatacenters] = await Promise.all([
          commands.regionSelectorStatus(),
          commands.regionSelectorDatacenters(),
        ]);
        setStatus(selectorStatus);
        setDatacenters(selectorDatacenters);
      } catch (error: unknown) {
        setMessage(`Region selector init failed: ${String(error)}`);
      }
    };
    void init();
  }, []);

  const searchGames = async () => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }
    setSearching(true);
    setMessage("");
    try {
      const results = (await commands.regionSelectorSearchGames(searchQuery)) as RegionGameSearchEntry[];
      setSearchResults(results || []);
    } catch (error: unknown) {
      setMessage(`Game search failed: ${String(error)}`);
    } finally {
      setSearching(false);
    }
  };

  const loadServers = async (reset: boolean) => {
    const placeId = Number(placeIdInput);
    if (!Number.isFinite(placeId) || placeId <= 0) {
      setMessage("Enter a valid place ID first");
      return;
    }
    setLoadingServers(true);
    setMessage("");
    try {
      const page = (await commands.regionSelectorServers(
        placeId,
        reset ? undefined : (nextCursor || undefined),
        sortOrder,
        s.SelectedRegion || undefined
      )) as RegionServerPage;

      if (reset) {
        setServers(page.data || []);
      } else {
        setServers((prev) => [...prev, ...(page.data || [])]);
      }
      setNextCursor(page.next_cursor || null);
      if ((page.data || []).length === 0) {
        setMessage("No servers matched this region on this page");
      }
    } catch (error: unknown) {
      setMessage(`Server fetch failed: ${String(error)}`);
    } finally {
      setLoadingServers(false);
    }
  };

  const joinServer = async (jobId: string) => {
    const placeId = Number(placeIdInput);
    if (!Number.isFinite(placeId) || placeId <= 0) return;
    try {
      await commands.regionSelectorJoin(placeId, jobId);
    } catch (error: unknown) {
      setMessage(`Join failed: ${String(error)}`);
    }
  };

  return (
    <Section title="Region Selector" desc="Search games, browse servers by region, and join directly.">
      <Opt header="Preferred region" desc="Used for server filtering and better matchmaking.">
        <select value={s.SelectedRegion || ""} onChange={(e) => set("SelectedRegion", e.target.value)}>
          <option value="">Automatic</option>
          {(datacenters?.regions || []).map((region) => (
            <option key={region} value={region}>{region}</option>
          ))}
        </select>
      </Opt>
      <div style={{ padding: 16, borderTop: "1px solid var(--border)", display: "grid", gap: 12 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 8 }}>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search games (e.g. bee garden)"
            onKeyDown={(e) => e.key === "Enter" && searchGames()}
          />
          <button className="btn-secondary" onClick={() => void searchGames()} disabled={searching}>
            {searching ? "Searching..." : "Search"}
          </button>
        </div>
        {searchResults.length > 0 && (
          <div style={{ display: "grid", gap: 6, maxHeight: 180, overflowY: "auto" }}>
            {searchResults.map((result) => (
              <button
                key={`${result.universe_id}-${result.root_place_id}`}
                className="btn-secondary"
                style={{ textAlign: "left", display: "flex", alignItems: "center", gap: 10 }}
                onClick={() => {
                  setPlaceIdInput(String(result.root_place_id));
                  setSearchQuery(result.name);
                }}
              >
                {result.thumbnail_url && (
                  <img
                    src={result.thumbnail_url}
                    alt={result.name}
                    width={34}
                    height={34}
                    style={{ borderRadius: 6, objectFit: "cover" }}
                  />
                )}
                <span style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                  <strong>{result.name}</strong>
                  <small style={{ color: "var(--text-secondary)" }}>Place {result.root_place_id}</small>
                </span>
              </button>
            ))}
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 160px auto", gap: 8 }}>
          <input
            type="number"
            value={placeIdInput}
            onChange={(e) => setPlaceIdInput(e.target.value)}
            placeholder="Place ID"
          />
          <select value={sortOrder} onChange={(e) => setSortOrder(Number(e.target.value))}>
            <option value={2}>Large servers</option>
            <option value={1}>Small servers</option>
          </select>
          <button className="btn-primary" onClick={() => void loadServers(true)} disabled={loadingServers}>
            {loadingServers ? "Loading..." : "Load Servers"}
          </button>
        </div>

        <div className="ff-table-wrap" style={{ maxHeight: 280 }}>
          <table className="ff-table">
            <thead>
              <tr>
                <th>Job ID</th>
                <th>Players</th>
                <th>Region</th>
                <th>Uptime</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {servers.map((server) => (
                <tr key={server.job_id}>
                  <td style={{ fontFamily: "monospace", fontSize: 11 }}>{server.job_id.slice(0, 12)}...</td>
                  <td>{server.playing}/{server.max_players}</td>
                  <td>{server.region}</td>
                  <td>{server.uptime || "N/A"}</td>
                  <td>
                    <button className="btn-secondary btn-sm" onClick={() => void joinServer(server.job_id)}>
                      Join
                    </button>
                  </td>
                </tr>
              ))}
              {servers.length === 0 && (
                <tr>
                  <td colSpan={5} style={{ textAlign: "center", padding: 16, opacity: 0.5 }}>
                    No servers loaded
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>
            Cookie: {status?.cookie_state || "unknown"} | Valid: {status?.has_valid_cookie ? "yes" : "no"}
          </span>
          <button
            className="btn-secondary"
            onClick={() => void loadServers(false)}
            disabled={loadingServers || !nextCursor}
          >
            Load More
          </button>
        </div>
        {message && <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{message}</div>}
      </div>
    </Section>
  );
}

/* ========================================
   PAGE: DEPLOYMENT
   ======================================== */

function PageDeployment({ s, set }: SettingsProps) {
  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Deployment</h2>
        <p>Change deployment and installation settings for Roblox &amp; Ruststrap.</p>
      </hgroup>

      <Section title="Ruststrap">
        <Opt header="Automatically update Ruststrap" desc="Check and update Ruststrap when launching Roblox.">
          <Toggle checked={s.CheckForUpdates} onChange={v => set("CheckForUpdates", v)} />
        </Opt>
      </Section>

      <Section title="Roblox">
        <Opt header="Force Roblox reinstallation" desc="Roblox will be installed fresh on next launch.">
          <Toggle checked={s.UpdateRoblox} onChange={v => set("UpdateRoblox", v)} />
        </Opt>
        <Opt header="Static directory" desc="Use BinaryType based install directories.">
          <Toggle checked={s.StaticDirectory} onChange={v => set("StaticDirectory", v)} />
        </Opt>
        <Opt header="Channel" desc="Choose deployment channel.">
          <input type="text" value={s.Channel || "LIVE"} onChange={e => set("Channel", e.target.value)} style={{ width: 140 }} />
        </Opt>
        <Opt header="Automatic channel change" desc="Action when Roblox tries to change your channel.">
          <select value={s.ChannelChangeMode} onChange={e => set("ChannelChangeMode", Number(e.target.value))}>
            <option value={0}>Always Prompt</option>
            <option value={1}>Always Allow</option>
            <option value={2}>Always Deny</option>
          </select>
        </Opt>
      </Section>
    </div>
  );
}

/* ========================================
   PAGE: MODS
   ======================================== */

function PageMods({ s, set }: SettingsProps) {
  const cursorTypes = [{ v: 0, l: "Default" }, { v: 1, l: "From 2006" }, { v: 2, l: "From 2013" }];
  const emojiTypes = [{ v: 0, l: "Default (Twemoji)" }, { v: 1, l: "Noto Color Emoji" }, { v: 2, l: "Windows 11" }];

  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Mods</h2>
        <p>Manage and apply file mods to the Roblox game client.</p>
      </hgroup>

      <CardGroup>
        <div style={{ display: "flex", gap: 8, padding: 14 }}>
          <button className="btn-secondary" style={{ flex: 1 }} onClick={() => void commands.openSettings()}>
            Open Mods Folder
          </button>
        </div>
      </CardGroup>

      <Section title="Presets">
        <Opt header="Mouse cursor" desc="Choose classic Roblox cursor styles.">
          <select value={(s.extra as Record<string, number>)?.CursorType ?? 0} onChange={e => set("extra" as keyof Settings, { ...(s.extra || {}), CursorType: Number(e.target.value) } as Settings["extra"])}>
            {cursorTypes.map(c => <option key={c.v} value={c.v}>{c.l}</option>)}
          </select>
        </Opt>
        <Opt header="Use old avatar editor background" desc="Bring back the pre-2020 avatar editor background.">
          <Toggle checked={(s.extra as Record<string, boolean>)?.OldAvatarBackground ?? false} onChange={v => set("extra" as keyof Settings, { ...(s.extra || {}), OldAvatarBackground: v } as Settings["extra"])} />
        </Opt>
        <Opt header="Emulate old character sounds" desc="Bring back pre-2014 character sounds.">
          <Toggle checked={(s.extra as Record<string, boolean>)?.OldCharacterSounds ?? false} onChange={v => set("extra" as keyof Settings, { ...(s.extra || {}), OldCharacterSounds: v } as Settings["extra"])} />
        </Opt>
        <Opt header="Preferred emoji type" desc="Choose which emoji Roblox should use.">
          <select value={(s.extra as Record<string, number>)?.EmojiType ?? 0} onChange={e => set("extra" as keyof Settings, { ...(s.extra || {}), EmojiType: Number(e.target.value) } as Settings["extra"])}>
            {emojiTypes.map(e => <option key={e.v} value={e.v}>{e.l}</option>)}
          </select>
        </Opt>
      </Section>

      <Section title="Miscellaneous">
        <Opt header="Use custom font" desc="Font override will be added in a future update.">
          <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Coming soon</span>
        </Opt>
        <Opt header="Manage compatibility settings" desc="Configure DPI scaling behavior.">
          <Toggle checked={s.WPFSoftwareRender} onChange={v => set("WPFSoftwareRender", v)} />
        </Opt>
      </Section>
    </div>
  );
}

/* ========================================
   PAGE: FAST FLAGS
   ======================================== */

function PageFastFlags({ flags, setFlags, s, set }: { flags: Record<string, string>; setFlags: React.Dispatch<React.SetStateAction<Record<string, string>>> } & SettingsProps) {
  const [newKey, setNewKey] = useState("");
  const [newVal, setNewVal] = useState("");
  const put = (k: string, v: string) => setFlags({ ...flags, [k]: v });

  const renderModes = ["Automatic", "Direct3D 11", "Direct3D 10", "OpenGL", "Vulkan", "Metal"];
  const msaaLevels = ["Automatic", "1x", "2x", "4x"];
  const textureQualities = ["Automatic", "Level 0", "Level 1", "Level 2", "Level 3"];

  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Fast Flags</h2>
        <p>Control Roblox engine parameters and features.</p>
      </hgroup>

      <WarningBanner>Roblox only applies whitelisted FFlags. Double click to learn more.</WarningBanner>

      <CardGroup>
        <Opt header="Allow Ruststrap to manage Fast Flags" desc="Disabling prevents configured flags from being applied.">
          <Toggle checked={s.UseFastFlagManager} onChange={v => set("UseFastFlagManager", v)} />
        </Opt>
      </CardGroup>

      <Section title="Presets - Geometry">
        <Opt header="Mesh detail" desc="Control how detailed meshes appear.">
          <select value={flags["FIntRenderMeshQuality"] ?? ""} onChange={e => put("FIntRenderMeshQuality", e.target.value)}>
            <option value="">Default</option>
            <option value="0">Lowest</option>
            <option value="1">Low</option>
            <option value="2">Medium</option>
            <option value="3">High</option>
          </select>
        </Opt>
      </Section>

      <Section title="Presets - Rendering">
        <Opt header="Anti-aliasing quality (MSAA)">
          <select value={flags["FIntDebugForceMSAASamples"] ?? ""} onChange={e => put("FIntDebugForceMSAASamples", e.target.value)}>
            {msaaLevels.map(l => <option key={l} value={l === "Automatic" ? "" : l.replace("x", "")}>{l}</option>)}
          </select>
        </Opt>
        <Opt header="Preserve quality with display scaling" desc="Prevent quality reduction based on Windows display scale.">
          <Toggle checked={flags["DFFlagDisableDPIScale"] === "true"} onChange={v => put("DFFlagDisableDPIScale", v ? "true" : "false")} />
        </Opt>
        <Opt header="Rendering mode">
          <select value={flags["FFlagDebugGraphicsPreferD3D11"] ?? ""} onChange={e => put("FFlagDebugGraphicsPreferD3D11", e.target.value)}>
            {renderModes.map(m => <option key={m} value={m === "Automatic" ? "" : m}>{m}</option>)}
          </select>
        </Opt>
        <Opt header="Texture quality">
          <select value={flags["FIntDebugTextureManagerSkipMips"] ?? ""} onChange={e => put("FIntDebugTextureManagerSkipMips", e.target.value)}>
            {textureQualities.map(q => <option key={q} value={q === "Automatic" ? "" : q.replace("Level ", "")}>{q}</option>)}
          </select>
        </Opt>
      </Section>

      <Section title="Custom Flags" desc="Add your own FFlags here." />
      <div className="ff-add-row">
        <input type="text" placeholder="Flag name" value={newKey} onChange={e => setNewKey(e.target.value)} style={{ flex: 1 }} />
        <input type="text" placeholder="Value" value={newVal} onChange={e => setNewVal(e.target.value)} style={{ flex: 1 }} />
        <button className="btn-secondary" onClick={() => { 
          if (newKey.trim()) { 
            put(newKey.trim(), newVal); 
            setNewKey(""); 
            setNewVal(""); 
          } 
        }}>
          <Plus size={14} /> Add
        </button>
      </div>
      <div className="ff-table-wrap">
        <table className="ff-table">
          <thead>
            <tr>
              <th>Flag</th>
              <th>Value</th>
              <th style={{ width: 40 }}></th>
            </tr>
          </thead>
          <tbody>
            {Object.entries(flags).map(([k, v]) => (
              <tr key={k}>
                <td>{k}</td>
                <td><input type="text" value={v} onChange={e => put(k, e.target.value)} /></td>
                <td className="ff-del" onClick={() => { const next = { ...flags }; delete next[k]; setFlags(next); }}>&#x2715;</td>
              </tr>
            ))}
            {Object.keys(flags).length === 0 && (
              <tr>
                <td colSpan={3} style={{ textAlign: "center", padding: 16, opacity: 0.5 }}>No custom flags</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/* ========================================
   PAGE: APPEARANCE
   ======================================== */

function PageAppearance({ s, set }: SettingsProps) {
  const themes = [{ v: 0, l: "Dark (Default)" }, { v: 1, l: "Light" }, { v: 2, l: "System" }];
  const bootstrapperStyles = [{ v: 0, l: "Progress Dialog" }, { v: 1, l: "Legacy" }, { v: 2, l: "Compact" }];
  const bootstrapperIcons = [{ v: 0, l: "Default" }, { v: 1, l: "Classic" }, { v: 2, l: "Custom" }];
  const robloxIcons = [{ v: 0, l: "Default" }, { v: 1, l: "Classic 2006" }, { v: 2, l: "Classic 2011" }, { v: 3, l: "Custom" }];

  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Appearance</h2>
        <p>Customize the look and feel of Ruststrap and Roblox.</p>
      </hgroup>

      <Section title="Ruststrap Theme">
        <Opt header="Theme" desc="Choose the Ruststrap UI theme.">
          <select value={s.Theme} onChange={e => set("Theme", Number(e.target.value))}>
            {themes.map(t => <option key={t.v} value={t.v}>{t.l}</option>)}
          </select>
        </Opt>
      </Section>

      <Section title="Bootstrapper">
        <Opt header="Bootstrapper style" desc="Change the appearance of the launch window.">
          <select value={s.BootstrapperStyle} onChange={e => set("BootstrapperStyle", Number(e.target.value))}>
            {bootstrapperStyles.map(bs => <option key={bs.v} value={bs.v}>{bs.l}</option>)}
          </select>
        </Opt>
        <Opt header="Bootstrapper icon" desc="Change the icon shown during launch.">
          <select value={s.BootstrapperIcon} onChange={e => set("BootstrapperIcon", Number(e.target.value))}>
            {bootstrapperIcons.map(bi => <option key={bi.v} value={bi.v}>{bi.l}</option>)}
          </select>
        </Opt>
        <Opt header="Bootstrapper title" desc="Custom title for the launch window.">
          <input type="text" value={s.BootstrapperTitle} onChange={e => set("BootstrapperTitle", e.target.value)} placeholder="Ruststrap" style={{ width: 160 }} />
        </Opt>
        {s.BootstrapperIcon === 2 && (
          <Opt header="Custom icon path" desc="Path to your custom bootstrapper icon.">
            <input type="text" value={s.BootstrapperIconCustomLocation} onChange={e => set("BootstrapperIconCustomLocation", e.target.value)} placeholder="C:\path\to\icon.ico" />
          </Opt>
        )}
      </Section>

      <Section title="Roblox">
        <Opt header="Roblox taskbar icon" desc="Change the Roblox icon in your taskbar.">
          <select value={s.RobloxIcon} onChange={e => set("RobloxIcon", Number(e.target.value))}>
            {robloxIcons.map(ri => <option key={ri.v} value={ri.v}>{ri.l}</option>)}
          </select>
        </Opt>
        <Opt header="Roblox window title" desc="Custom title for the Roblox window.">
          <input type="text" value={s.RobloxTitle} onChange={e => set("RobloxTitle", e.target.value)} placeholder="Roblox" style={{ width: 160 }} />
        </Opt>
        {s.RobloxIcon === 3 && (
          <Opt header="Custom icon path" desc="Path to your custom Roblox icon.">
            <input type="text" value={s.RobloxIconCustomLocation} onChange={e => set("RobloxIconCustomLocation", e.target.value)} placeholder="C:\path\to\icon.ico" />
          </Opt>
        )}
      </Section>
    </div>
  );
}

/* ========================================
   PAGE: SHORTCUTS
   ======================================== */

function PageShortcuts() {
  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>Shortcuts</h2>
        <p>Manage keyboard shortcuts and quick actions.</p>
      </hgroup>

      <CardGroup>
        <div style={{ padding: 20, textAlign: "center", opacity: 0.5, fontSize: 13 }}>
          Keyboard shortcuts coming in a future update
        </div>
      </CardGroup>
    </div>
  );
}

/* ========================================
   PAGE: ABOUT
   ======================================== */

function PageAbout() {
  return (
    <div className="page">
      <hgroup className="page-header">
        <h2>About</h2>
        <p>Information about Ruststrap.</p>
      </hgroup>

      <CardGroup>
        <div style={{ padding: 20 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 16 }}>
            <img src="/icon.png" alt="Ruststrap" width={48} height={48} style={{ borderRadius: 10 }} />
            <div>
              <h3 style={{ fontSize: 16, fontWeight: 600, marginBottom: 4 }}>Ruststrap</h3>
              <p style={{ fontSize: 13, color: "var(--text-secondary)" }}>A high-performance Roblox bootstrapper</p>
            </div>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn-secondary" onClick={() => window.open("https://github.com/Ruststrap/Ruststrap")}>
              GitHub
            </button>
            <button className="btn-secondary" onClick={() => window.open("https://discord.gg/KdR9vpRcUN")}>
              Discord
            </button>
            <button className="btn-secondary" onClick={() => window.open("https://github.com/Ruststrap/Ruststrap/wiki")}>
              Wiki
            </button>
          </div>
        </div>
      </CardGroup>

      <Section title="Contributors">
        <div style={{ padding: 16, fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.6 }}>
          Thanks to all the contributors who have helped make Ruststrap possible.
        </div>
      </Section>
    </div>
  );
}
