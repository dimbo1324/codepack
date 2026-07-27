// Number, size and time formatting shared by every page.
//
// Kept in one module rather than per page because two pages formatting the same quantity
// two ways is how a UI starts contradicting itself — the analytics page and the preview
// page must not disagree about what "1.5 MB" means.
import type { Language } from "$lib/i18n/index.svelte";

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Binary units, mirroring `codepack_tokens::format_bytes` exactly — same base, same
 * two-decimal precision, same truncation for whole bytes — so the window and the reports
 * inside the bundle quote one number one way. */
export function formatBytes(size: number): string {
  let value = size;
  for (let index = 0; index < BYTE_UNITS.length; index += 1) {
    if (value < 1024 || index === BYTE_UNITS.length - 1) {
      return index === 0
        ? `${Math.trunc(value)} ${BYTE_UNITS[index]}`
        : `${value.toFixed(2)} ${BYTE_UNITS[index]}`;
    }
    value /= 1024;
  }
  return `${size} B`;
}

/** Thousands-grouped, so a six-digit file count is readable at a glance. */
export function formatCount(value: number, language: Language): string {
  return new Intl.NumberFormat(language === "ru" ? "ru-RU" : "en-US").format(value);
}

/** Compact token counts: 12 400 tokens says less than "12.4K" does at stat-tile size. */
export function formatCompact(value: number, language: Language): string {
  return new Intl.NumberFormat(language === "ru" ? "ru-RU" : "en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

/** `mm:ss`, or `h:mm:ss` past an hour. Used for a running export's elapsed time. */
export function formatDuration(milliseconds: number): string {
  const total = Math.max(0, Math.floor(milliseconds / 1000));
  const seconds = total % 60;
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);
  const pad = (part: number) => String(part).padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 7],
  ["week", 4.348],
  ["month", 12],
  ["year", Number.POSITIVE_INFINITY],
];

/** "3 minutes ago" from a unix-seconds timestamp. The absolute UTC string stays
 * available as the row's tooltip: relative time is what a user scans, absolute time is
 * what they need when they are actually comparing two runs. */
export function formatRelativeTime(unixSeconds: number, language: Language): string {
  const formatter = new Intl.RelativeTimeFormat(language === "ru" ? "ru-RU" : "en-US", {
    numeric: "auto",
  });
  let delta = unixSeconds - Date.now() / 1000;
  for (const [unit, step] of RELATIVE_UNITS) {
    if (Math.abs(delta) < step || step === Number.POSITIVE_INFINITY) {
      return formatter.format(Math.round(delta), unit);
    }
    delta /= step;
  }
  return formatter.format(Math.round(delta), "year");
}

/** The last path segment, for showing a project by name without losing the full path in
 * a tooltip. Handles both separators because a Windows build still sees POSIX paths in
 * history rows written on another machine. */
export function baseName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const index = Math.max(trimmed.lastIndexOf("\\"), trimmed.lastIndexOf("/"));
  return index === -1 ? trimmed : trimmed.slice(index + 1);
}
