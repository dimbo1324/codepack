import { DEFAULT_ARCHIVE_FORMAT, type ArchiveFormat } from "$lib/util/archiveFormats";

// The wizard's shared state: which step is active, the project and its session
// configuration, and the last result of each step's own action. One store rather
// than per-page component state, because later steps (Preview, Export, Result) all
// need to read decisions made on earlier ones (Settings, Security).
import type {
  Config,
  ExportProgressEvent,
  ExportReport,
  PreviewReport,
  ProjectContext,
  SanitizeReport,
  ScanReport,
} from "$lib/api/types";
import { parseStep, PIPELINE_STEP_COUNT } from "$lib/util/pipeline";

export const STEPS = [
  "project",
  "settings",
  "security",
  "preview",
  "export",
  "result",
  "history",
  "analytics",
  "sterile",
] as const;

export type Step = (typeof STEPS)[number];

/** The six steps that form the actual export journey, in order. The remaining two
 * ("history", "analytics") are reference views, reachable at any time rather than
 * positions in a sequence, and numbering them alongside the others implied a progression
 * that does not exist. */
export const WIZARD_STEPS: Step[] = ["project", "settings", "security", "preview", "export"];

export const INSIGHT_STEPS: Step[] = ["result", "history", "analytics"];

/** "Sterile copy" is its own standalone action, not a wizard step or an insight view: it
 * has its own source folder (not necessarily the project the wizard has open) and its
 * own result (a folder of cleaned source, not an archive or a set of reports) — the
 * placement the stage planner recorded on 2026-07-28. */
export const STANDALONE_STEPS: Step[] = ["sterile"];

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
  /** The config the current preview was built from, so the page can say "these numbers
   * are stale" instead of quietly showing counts that no longer match the settings. */
  previewConfigStamp = $state<string | null>(null);

  scan = $state<ScanReport | null>(null);
  scanLoading = $state(false);

  /** Chosen once and kept for the session. It used to live inside the log page, so
   * navigating away and back silently forgot it and re-disabled the export button. */
  outputDir = $state<string | null>(null);

  exportRunId = $state<string | null>(null);
  exportRunning = $state(false);
  exportCancelling = $state(false);
  exportLog = $state<string[]>([]);
  /** Lines trimmed off the front of `exportLog` once it hit its cap. Counted so the page
   * can say so: a log that silently loses its beginning is worse than a short one. */
  exportLogDropped = $state(0);
  exportResult = $state<ExportReport | null>(null);
  exportError = $state<string | null>(null);
  exportStartedAt = $state<number | null>(null);
  exportFinishedAt = $state<number | null>(null);
  /** How many pipeline steps the engine reported finished, and which one it reported
   * started and not yet finished. Derived from the events alone — the UI never guesses a
   * step is done because time passed. */
  exportStepsDone = $state(0);
  exportStepCurrent = $state<number | null>(null);
  /** The denominator the engine itself reported ("1/8" → 8). Taken from the events
   * rather than assumed, so a pipeline that grows a ninth step reports "3 of 9" instead
   * of quietly lying about the one number the progress bar is built on. */
  exportStepTotal = $state(PIPELINE_STEP_COUNT);

  watchActive = $state(false);
  watchChangedPaths = $state<string[]>([]);

  /** State for the standalone "Sterile copy" action. Kept separate from the export
   * journey's fields above: its source is any folder the user picks, not necessarily
   * `wizard.project`, and it produces no archive/report artifacts of its own. */
  sterileSource = $state<string | null>(null);
  sterileDestination = $state<string | null>(null);
  /** Where to also write an archive of the sterile copy. `null` means folder only. */
  sterileArchive = $state<string | null>(null);
  /** Container for that archive. ZIP by default, like everywhere else. */
  sterileArchiveFormat = $state<ArchiveFormat>(DEFAULT_ARCHIVE_FORMAT);
  sterileSafetyMode = $state("safe");
  sterileRunId = $state<string | null>(null);
  sterileRunning = $state(false);
  sterileResult = $state<SanitizeReport | null>(null);
  sterileError = $state<string | null>(null);
}

export const wizard = new WizardState();

export function goTo(step: Step): void {
  wizard.step = step;
}

/** Why a step cannot be opened yet, or `null` when it can. Returning the reason rather
 * than a bare boolean is the point: a disabled navigation item that does not say what is
 * missing makes the user hunt for the prerequisite. */
export function blockedReason(step: Step): "project" | "export" | null {
  if (step === "project" || step === "sterile") return null;
  if (!wizard.project) return "project";
  if ((step === "result" || step === "analytics") && !wizard.exportResult) return "export";
  return null;
}

export function resetForNewProject(project: ProjectContext): void {
  wizard.project = project;
  wizard.sessionConfig = project.config;
  wizard.fileOverrides = {};
  wizard.preview = null;
  wizard.previewConfigStamp = null;
  wizard.scan = null;
  wizard.outputDir = null;
  resetExportState();
  wizard.step = "settings";
}

export function resetExportState(): void {
  wizard.exportRunId = null;
  wizard.exportRunning = false;
  wizard.exportCancelling = false;
  wizard.exportLog = [];
  wizard.exportLogDropped = 0;
  wizard.exportResult = null;
  wizard.exportError = null;
  wizard.exportStartedAt = null;
  wizard.exportFinishedAt = null;
  wizard.exportStepsDone = 0;
  wizard.exportStepCurrent = null;
  wizard.exportStepTotal = PIPELINE_STEP_COUNT;
  earlyEvents = {};
}

/** Progress events that arrived before the UI learned the run's id.
 *
 * `start_export` registers the run and spawns the pipeline *before* the command returns,
 * so the engine can emit "1/8: plan" while the `invoke()` promise is still in flight.
 * Filtering on a run id that is still `null` discarded exactly those events — including
 * the one that first lights up the progress indicator. Buffering by id rather than
 * accepting whatever arrives keeps a previous, still-finishing run from being adopted as
 * this one. */
let earlyEvents: Record<string, ExportProgressEvent[]> = {};

const EARLY_EVENT_LIMIT = 512;

export function receiveExportEvent(event: ExportProgressEvent): void {
  if (wizard.exportRunId === null) {
    if (!wizard.exportRunning) return;
    const buffered = earlyEvents[event.run_id] ?? [];
    if (buffered.length < EARLY_EVENT_LIMIT) buffered.push(event);
    earlyEvents[event.run_id] = buffered;
    return;
  }
  if (event.run_id !== wizard.exportRunId) return;
  applyExportEvent(event);
}

/** Called once `start_export` has returned an id: adopts it and replays whatever the
 * engine already sent for that run, in arrival order. */
export function adoptExportRun(runId: string): void {
  wizard.exportRunId = runId;
  const buffered = earlyEvents[runId] ?? [];
  earlyEvents = {};
  for (const event of buffered) applyExportEvent(event);
}

const LOG_LIMIT = 5000;

function applyExportEvent(event: ExportProgressEvent): void {
  if (event.message) {
    // Bounded: a large project can emit far more lines than anyone will read, and the
    // page re-joins the whole buffer on every render. Dropped lines are counted and
    // shown rather than silently discarded.
    const next = [...wizard.exportLog, event.message];
    if (next.length > LOG_LIMIT) {
      wizard.exportLogDropped += next.length - LOG_LIMIT;
      wizard.exportLog = next.slice(next.length - LOG_LIMIT);
    } else {
      wizard.exportLog = next;
    }
  }
  applyStepEvent(event.step, event.step_finished);
}

/** Folds one progress event into the run's step counters. Kept here rather than in the
 * root component so the rule lives next to the state it owns. */
function applyStepEvent(label: string | null, finished: boolean): void {
  const parsed = parseStep(label);
  if (!parsed) return;
  wizard.exportStepTotal = parsed.total;
  if (finished) {
    wizard.exportStepsDone = Math.max(wizard.exportStepsDone, parsed.ordinal);
    if (wizard.exportStepCurrent === parsed.ordinal) wizard.exportStepCurrent = null;
  } else {
    wizard.exportStepCurrent = parsed.ordinal;
  }
}

/** A cheap fingerprint of the settings a preview depends on. Not every config field
 * changes the plan, but the ones listed here do, and over-reporting staleness is the
 * safe direction: the cost is one extra click, not a wrong file list. */
export function previewStamp(config: Config, overrides: Record<string, boolean>): string {
  return JSON.stringify([
    config.export_profile,
    config.safe_export_mode,
    config.diff_export_mode,
    config.diff_base_ref,
    config.diff_target_ref,
    config.token_budget,
    config.custom_excluded_files,
    config.custom_excluded_extensions,
    config.extra_ignored_dirs,
    config.always_include_files,
    config.always_include_dirs,
    Object.entries(overrides).sort(),
  ]);
}
