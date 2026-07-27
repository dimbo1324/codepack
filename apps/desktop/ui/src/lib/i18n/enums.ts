// Backend enum values, translated for display.
//
// `Finding.severity`, `Config.safe_export_mode`, `diff_export_mode` and
// `ProjectProfileSummary.risk_level` all cross the IPC boundary as bare identifiers —
// they are contract values, not prose, so the Rust side is right to send `git_ref` rather
// than a sentence. Rendering them raw, though, left the only English words on an
// otherwise Russian screen.
//
// Every lookup falls back to the identifier itself. A value this table has not heard of
// must still be shown: inventing a label for it, or hiding it, would misreport what the
// backend actually said.
import type { TranslationKey } from "./en";
import { t } from "./index.svelte";

function translate(table: Record<string, TranslationKey>, value: string): string {
  const key = table[value];
  return key ? t(key) : value;
}

const SEVERITY: Record<string, TranslationKey> = {
  critical: "severity.critical",
  high: "severity.high",
  medium: "severity.medium",
  low: "severity.low",
  warning: "severity.warning",
};

const CONFIDENCE: Record<string, TranslationKey> = {
  high: "confidence.high",
  medium: "confidence.medium",
  low: "confidence.low",
};

const SAFE_MODE: Record<string, TranslationKey> = {
  safe: "security.safeMode.safe",
  balanced: "security.safeMode.balanced",
  full: "security.safeMode.full",
};

const DIFF_MODE: Record<string, TranslationKey> = {
  all: "settings.diffMode.all",
  last_export: "settings.diffMode.lastExport",
  git_ref: "settings.diffMode.gitRef",
  uncommitted: "settings.diffMode.uncommitted",
};

const RISK: Record<string, TranslationKey> = {
  low: "risk.low",
  medium: "risk.medium",
  high: "risk.high",
  critical: "risk.critical",
};

export const severityLabel = (value: string): string => translate(SEVERITY, value);
export const confidenceLabel = (value: string): string => translate(CONFIDENCE, value);
export const safeModeLabel = (value: string): string => translate(SAFE_MODE, value);
export const diffModeLabel = (value: string): string => translate(DIFF_MODE, value);
export const riskLabel = (value: string): string => translate(RISK, value);
