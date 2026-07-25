// The single choke point through which the frontend talks to the Rust backend.
//
// Invariant (`ROADMAP.md` §3: "изоляция — UI не имеет прямого доступа к ФС"): no
// other module in this package may import `@tauri-apps/api/core` or a filesystem
// plugin directly. Every file operation the UI needs — picking a folder, reading a
// project, writing an archive — is a named function here, backed by a
// `#[tauri::command]`, never a raw `invoke()` call scattered through page code.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

import type {
  AppInfo,
  Config,
  ExportFinishedEvent,
  ExportProgressEvent,
  HistoryReport,
  PreviewReport,
  ProjectContext,
  ProjectProfileSummary,
  ScanReport,
} from "./types";

/** Opens the native folder picker. `null` means the user cancelled — not an error. */
export async function pickProjectDirectory(): Promise<string | null> {
  const selected = await openDialog({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke("get_app_info");
}

export function loadGlobalSettings(): Promise<Config> {
  return invoke("load_global_settings");
}

export function saveGlobalSettings(config: Config): Promise<void> {
  return invoke("save_global_settings", { config });
}

/** Applies a built-in AI preset's fields onto `config`, returning the updated copy.
 * Goes through the backend rather than being reimplemented in TypeScript so the GUI
 * can never disagree with the CLI about what a preset actually sets
 * (`codepack_core::config::ai_presets`, the single source of truth for both). */
export function applyPreset(config: Config, presetName: string): Promise<Config> {
  return invoke("apply_preset", { config, presetName });
}

/** Same reasoning as `applyPreset`, for a built-in or user-defined export profile
 * (`codepack_core::profiles::apply_custom_profile`). */
export function applyProfile(config: Config, profileName: string): Promise<Config> {
  return invoke("apply_profile", { config, profileName });
}

/** Resolves defaults -> global settings -> `.codepack.toml` for `path`. The result is
 * the starting point for the session's editable config; nothing here is persisted. */
export function openProject(path: string): Promise<ProjectContext> {
  return invoke("open_project", { path });
}

export function previewProject(
  projectRoot: string,
  config: Config,
  fileOverrides: Record<string, boolean>,
): Promise<PreviewReport> {
  return invoke("preview_project", { projectRoot, config, fileOverrides });
}

export function scanProject(projectRoot: string, config: Config): Promise<ScanReport> {
  return invoke("scan_project", { projectRoot, config });
}

/** Starts an export on a background thread and returns immediately with a run id.
 * Progress and completion arrive as events (`onExportProgress`/`onExportFinished`),
 * not as this call's return value — an export can run for minutes, and the UI must
 * stay responsive and cancellable while it does. */
export function startExport(
  projectRoot: string,
  outDir: string,
  config: Config,
  fileOverrides: Record<string, boolean>,
): Promise<string> {
  return invoke("start_export", { projectRoot, outDir, config, fileOverrides });
}

export function cancelExport(runId: string): Promise<void> {
  return invoke("cancel_export", { runId });
}

export function onExportProgress(
  handler: (event: ExportProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ExportProgressEvent>("export:progress", (event) => handler(event.payload));
}

export function onExportFinished(
  handler: (event: ExportFinishedEvent) => void,
): Promise<UnlistenFn> {
  return listen<ExportFinishedEvent>("export:finished", (event) => handler(event.payload));
}

export function fetchHistory(projectRoot: string | null, limit: number): Promise<HistoryReport> {
  return invoke("fetch_history", { projectRoot, limit });
}

export function readProjectProfile(resultPath: string): Promise<ProjectProfileSummary> {
  return invoke("read_project_profile", { resultPath });
}

/** Extracts the full report bundle from `resultPath` next to it and opens
 * `REPORT_DASHBOARD.html` with the OS's default handler, so its relative links to
 * the other reports resolve. */
export function openDashboard(resultPath: string): Promise<void> {
  return invoke("open_dashboard", { resultPath });
}

/** Same extraction, but for `PROJECT_OVERVIEW.html` — the plain-language overview
 * (stage S12, BLUEPRINT §B.9). */
export function openProjectOverview(resultPath: string): Promise<void> {
  return invoke("open_project_overview", { resultPath });
}

/** Same extraction, but for `ONBOARDING_GUIDE.md` (stage S12). */
export function openOnboardingGuide(resultPath: string): Promise<void> {
  return invoke("open_onboarding_guide", { resultPath });
}

/** Same extraction, but for `REVIEW_CHECKLIST.md` (stage S12). */
export function openReviewChecklist(resultPath: string): Promise<void> {
  return invoke("open_review_checklist", { resultPath });
}

export function openResultLocation(resultPath: string): Promise<void> {
  return revealItemInDir(resultPath);
}

export function openFile(path: string): Promise<void> {
  return openPath(path);
}

// --- Watch mode ----------------------------------------------------------------

export function startWatch(projectRoot: string, config: Config): Promise<void> {
  return invoke("start_watch", { projectRoot, config });
}

export function stopWatch(): Promise<void> {
  return invoke("stop_watch");
}

export interface WatchChangedEvent {
  changed_paths: string[];
}

export function onWatchChanged(handler: (event: WatchChangedEvent) => void): Promise<UnlistenFn> {
  return listen<WatchChangedEvent>("watch:changed", (event) => handler(event.payload));
}

// --- Window chrome ---------------------------------------------------------------

export function setUiZoom(factor: number): Promise<void> {
  return invoke("set_ui_zoom", { factor });
}
