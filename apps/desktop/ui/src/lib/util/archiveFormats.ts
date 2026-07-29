import type { SegmentOption } from "$lib/components/Segmented.svelte";
import { t } from "$lib/i18n/index.svelte";

/** Mirrors `codepack_core::config::ARCHIVE_FORMATS`. ZIP first because it is the
 * default and what every earlier release produced. */
export type ArchiveFormat = "zip" | "7z" | "rar";

export const DEFAULT_ARCHIVE_FORMAT: ArchiveFormat = "zip";

/** The one place the format choice is described, so the export wizard and the sterile
 * copy can never drift into offering different lists.
 *
 * RAR is listed and dimmed rather than omitted: the choice genuinely exists and is not
 * built yet, and a user who wonders whether it is coming deserves an answer on screen
 * rather than silence. `unavailable` makes `Segmented` show the reason when they try. */
export function archiveFormatOptions(): SegmentOption<ArchiveFormat>[] {
  return [
    {
      value: "zip",
      label: t("archive.format.zip"),
      hint: t("archive.format.hint.zip"),
    },
    {
      value: "7z",
      label: t("archive.format.sevenZip"),
      hint: t("archive.format.hint.sevenZip"),
    },
    {
      value: "rar",
      label: t("archive.format.rar"),
      unavailable: t("archive.format.rar.unavailable"),
    },
  ];
}

/** The extension a saved archive should carry, given the chosen format. */
export function archiveExtension(format: ArchiveFormat): string {
  return format;
}
