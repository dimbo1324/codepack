// The wizard's shared state: which step is active, the project and its session
// configuration, and the last result of each step's own action. One store rather
// than per-page component state, because later steps (Preview, Export, Result) all
// need to read decisions made on earlier ones (Settings, Security).
import type {
  Config,
  ExportReport,
  PreviewReport,
  ProjectContext,
  ScanReport,
} from "$lib/api/types";

export const STEPS = [
  "project",
  "settings",
  "security",
  "preview",
  "log",
  "result",
  "history",
  "analytics",
] as const;

export type Step = (typeof STEPS)[number];

class WizardState {
  step = $state<Step>("project");

  project = $state<ProjectContext | null>(null);
  /** The config this session is actually using: `project.config` as loaded, then
   * whatever the Settings/Security pages change. Never written back to the global
   * settings file unless the user explicitly asks (`settings.save`). */
  sessionConfig = $state<Config | null>(null);
  /** Per-file overrides from the Preview tree's "force include/exclude" actions,
   * keyed by backslash-joined relative path (matching `codepack_scanner`'s own
   * convention, so no translation happens at the Rust boundary). */
  fileOverrides = $state<Record<string, boolean>>({});

  preview = $state<PreviewReport | null>(null);
  previewLoading = $state(false);

  scan = $state<ScanReport | null>(null);
  scanLoading = $state(false);

  exportRunId = $state<string | null>(null);
  exportRunning = $state(false);
  exportCancelling = $state(false);
  exportLog = $state<string[]>([]);
  exportResult = $state<ExportReport | null>(null);
  exportError = $state<string | null>(null);
}

export const wizard = new WizardState();

export function goTo(step: Step): void {
  wizard.step = step;
}

export function resetForNewProject(project: ProjectContext): void {
  wizard.project = project;
  wizard.sessionConfig = project.config;
  wizard.fileOverrides = {};
  wizard.preview = null;
  wizard.scan = null;
  wizard.exportRunId = null;
  wizard.exportRunning = false;
  wizard.exportCancelling = false;
  wizard.exportLog = [];
  wizard.exportResult = null;
  wizard.exportError = null;
  wizard.step = "settings";
}
