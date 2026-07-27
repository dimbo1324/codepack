// The export pipeline's eight steps, as the UI needs to show them.
//
// The backend already reports step boundaries — `ExportProgressEvent.step` carries
// strings shaped `"1/8: plan"` and `step_finished` says which edge it is — and the
// previous UI dropped both on the floor, leaving the user with an undifferentiated log
// tail as the only sign that a multi-minute operation was alive.
//
// The ordinal is parsed out of the label rather than matched against a hard-coded list of
// names: a renamed step then still lands in the right slot, and an unrecognised string
// degrades to "no step highlighted" instead of freezing the indicator on step 1.
import type { TranslationKey } from "$lib/i18n/en";

export const PIPELINE_STEP_KEYS: TranslationKey[] = [
  "pipeline.step.plan",
  "pipeline.step.copy",
  "pipeline.step.structure",
  "pipeline.step.git",
  "pipeline.step.textDump",
  "pipeline.step.analytics",
  "pipeline.step.manifest",
  "pipeline.step.archive",
];

export const PIPELINE_STEP_COUNT = PIPELINE_STEP_KEYS.length;

export interface ParsedStep {
  /** 1-based, matching the label the engine emits. */
  ordinal: number;
  total: number;
}

const STEP_PATTERN = /^\s*(\d+)\s*\/\s*(\d+)/;

export function parseStep(label: string | null): ParsedStep | null {
  if (!label) return null;
  const match = STEP_PATTERN.exec(label);
  if (!match) return null;
  const ordinal = Number(match[1]);
  const total = Number(match[2]);
  if (!Number.isFinite(ordinal) || !Number.isFinite(total) || ordinal < 1) return null;
  return { ordinal, total };
}

export type StepState = "done" | "running" | "pending";

/** `completed` counts steps the engine said it finished; `current` is the step it said it
 * started and has not finished. A step can be neither when a run is between boundaries. */
export function stepState(ordinal: number, completed: number, current: number | null): StepState {
  if (ordinal <= completed) return "done";
  if (current !== null && ordinal === current) return "running";
  return "pending";
}
