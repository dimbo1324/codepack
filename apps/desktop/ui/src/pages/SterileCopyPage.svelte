<script lang="ts">
  // "Sterile copy": a standalone action, not a step of the export wizard — its own
  // source folder (not necessarily `wizard.project`), its own destination folder, and
  // its own result (cleaned source, not an archive or reports). See `nav.section.tools`
  // in the sidebar and `docs/__arch__/open-questions.md` (2026-07-28) for why this is a
  // separate page rather than an option inside Export.
  //
  // Unlike the Export page, there is no step-by-step progress: `codepack-sanitize`
  // processes the whole file set in one pass and reports only the finished event.
  import {
    cancelSanitize,
    onSanitizeFinished,
    onWindowDragDrop,
    pickArchiveDestination,
    pickProjectDirectory,
    openResultLocation,
    startSanitize,
  } from "$lib/api/client";
  import type { SanitizeFileOutcome, SanitizeOutcome } from "$lib/api/types";
  import Callout from "$lib/components/Callout.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Segmented, { type SegmentOption } from "$lib/components/Segmented.svelte";
  import Stat from "$lib/components/Stat.svelte";
  import type { TranslationKey } from "$lib/i18n/en";
  import { t } from "$lib/i18n/index.svelte";
  import { reportError } from "$lib/stores/toasts.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";
  import { formatBytes } from "$lib/util/format";
  import { archiveFormatOptions } from "$lib/util/archiveFormats";

  import { onMount } from "svelte";

  const safeModes: SegmentOption<string>[] = $derived([
    {
      value: "safe",
      label: t("security.safeMode.safe"),
      hint: t("security.safeMode.hint.safe"),
    },
    {
      value: "balanced",
      label: t("security.safeMode.balanced"),
      hint: t("security.safeMode.hint.balanced"),
    },
    {
      value: "full",
      label: t("security.safeMode.full"),
      hint: t("security.safeMode.hint.full"),
    },
  ]);

  const archiveFormats = $derived(archiveFormatOptions());

  const outcomeLabels: Record<SanitizeOutcome, TranslationKey> = {
    stripped_and_formatted: "sterile.outcome.stripped_and_formatted",
    stripped_only_no_formatter_found: "sterile.outcome.stripped_only_no_formatter_found",
    skipped_unsupported_language: "sterile.outcome.skipped_unsupported_language",
    skipped_sensitive_or_redacted: "sterile.outcome.skipped_sensitive_or_redacted",
    error: "sterile.outcome.error",
  };

  const outcomeTone: Record<SanitizeOutcome, "success" | "muted" | "warning" | "danger"> = {
    stripped_and_formatted: "success",
    stripped_only_no_formatter_found: "muted",
    skipped_unsupported_language: "muted",
    skipped_sensitive_or_redacted: "warning",
    error: "danger",
  };

  let sourceDragging = $state(false);

  // Registered/torn down with the component's own lifecycle rather than with an
  // explicit route check: `App.svelte` only mounts this page while
  // `wizard.step === "sterile"`, so listening here already means "only while this
  // page is the active view" — the same guarantee the sidebar-driven pages rely on
  // for `onSanitizeFinished` below. `App.svelte`'s own drag-and-drop handler stays
  // silent while this page is active, so a single drop is acted on exactly once.
  onMount(() => {
    let unlistenFinished: (() => void) | undefined;
    let unlistenDrop: (() => void) | undefined;
    (async () => {
      unlistenFinished = await onSanitizeFinished((event) => {
        if (event.run_id !== wizard.sterileRunId) return;
        wizard.sterileRunning = false;
        wizard.sterileResult = event.report;
        wizard.sterileError = event.error;
      });
      unlistenDrop = await onWindowDragDrop((phase, paths) => {
        if (wizard.sterileRunning) return;
        if (phase === "enter") {
          sourceDragging = true;
        } else if (phase === "leave") {
          sourceDragging = false;
        } else {
          sourceDragging = false;
          // Same single-path rule as the project-open drop handler: take the first
          // path, never guess among several.
          const dropped = paths[0];
          if (dropped) wizard.sterileSource = dropped;
        }
      });
    })();
    return () => {
      unlistenFinished?.();
      unlistenDrop?.();
    };
  });

  async function chooseSource(): Promise<void> {
    const chosen = await pickProjectDirectory();
    if (chosen) wizard.sterileSource = chosen;
  }

  async function chooseDestination(): Promise<void> {
    const chosen = await pickProjectDirectory();
    if (chosen) wizard.sterileDestination = chosen;
  }

  /** Suggests `<source folder name>-sterile.<ext>`, so the saved file says which
   * project it came from without the user having to type it. */
  function suggestedArchiveName(): string {
    const source = wizard.sterileSource ?? "";
    const leaf = source.split(/[\\/]/).filter(Boolean).pop() ?? "sterile-copy";
    return `${leaf}-sterile.${wizard.sterileArchiveFormat}`;
  }

  async function chooseArchive(): Promise<void> {
    const chosen = await pickArchiveDestination(
      suggestedArchiveName(),
      wizard.sterileArchiveFormat,
    );
    if (chosen) wizard.sterileArchive = chosen;
  }

  /** Retargets an already-chosen file when the format changes, so the extension never
   * contradicts the container. Silently writing `x.zip` as a 7z is the kind of small
   * lie a user only finds out about when something refuses to open it. */
  function onFormatChange(format: string): void {
    wizard.sterileArchiveFormat = format as typeof wizard.sterileArchiveFormat;
    if (wizard.sterileArchive) {
      wizard.sterileArchive = wizard.sterileArchive.replace(/\.[^.\\/]+$/, `.${format}`);
    }
  }

  function clearArchive(): void {
    wizard.sterileArchive = null;
  }

  async function revealArchive(path: string): Promise<void> {
    try {
      await openResultLocation(path);
    } catch (error) {
      reportError("sterile.revealFailed", error);
    }
  }

  async function begin(): Promise<void> {
    if (!wizard.sterileSource || !wizard.sterileDestination) return;
    wizard.sterileResult = null;
    wizard.sterileError = null;
    wizard.sterileRunning = true;
    try {
      wizard.sterileRunId = await startSanitize(
        wizard.sterileSource,
        wizard.sterileDestination,
        wizard.sterileSafetyMode,
        wizard.sterileArchive,
        wizard.sterileArchiveFormat,
      );
    } catch (error) {
      wizard.sterileRunning = false;
      reportError("sterile.startFailed", error);
    }
  }

  async function stop(): Promise<void> {
    if (!wizard.sterileRunId) return;
    try {
      await cancelSanitize(wizard.sterileRunId);
    } catch (error) {
      reportError("sterile.cancelFailed", error);
    }
  }

  /** The reason in the user's own language. Proper nouns — a language, a formatter —
   * come from the backend and stay as they are, because "Rust" and "rustfmt" are names,
   * not words to translate. */
  function detailOf(file: SanitizeFileOutcome): string | null {
    switch (file.detail_kind) {
      case "formatted_by":
        return file.detail ? t("sterile.detail.formattedBy", { detail: file.detail }) : null;
      case "no_formatter":
        return file.detail ? t("sterile.detail.noFormatter", { language: file.detail }) : null;
      case "unsupported_language":
        return t("sterile.detail.unsupportedLanguage");
      case "error":
        return file.detail;
      default:
        return null;
    }
  }
</script>

<div class="stack page">
  <div class="page-header">
    <div>
      <h1 class="page-title">{t("sterile.title")}</h1>
      <p class="page-lede">{t("sterile.lede")}</p>
    </div>
  </div>

  <section class="card">
    <div class="card__body stack">
      <div class="picker-row" class:picker-row--dragging={sourceDragging}>
        <span class="picker-row__icon"><Icon name="folder" size={16} /></span>
        <div class="picker-row__text">
          <p class="picker-row__label">{t("sterile.source")}</p>
          {#if wizard.sterileSource}
            <p class="path selectable">{wizard.sterileSource}</p>
          {:else}
            <p class="text-muted text-sm">{t("sterile.source.missing")}</p>
          {/if}
        </div>
        <button class="btn" onclick={chooseSource} disabled={wizard.sterileRunning}>
          {wizard.sterileSource ? t("sterile.source.change") : t("sterile.source.choose")}
        </button>
      </div>

      <div class="picker-row">
        <span class="picker-row__icon"><Icon name="download" size={16} /></span>
        <div class="picker-row__text">
          <p class="picker-row__label">{t("sterile.destination")}</p>
          {#if wizard.sterileDestination}
            <p class="path selectable">{wizard.sterileDestination}</p>
          {:else}
            <p class="text-muted text-sm">{t("sterile.destination.missing")}</p>
          {/if}
        </div>
        <button class="btn" onclick={chooseDestination} disabled={wizard.sterileRunning}>
          {wizard.sterileDestination
            ? t("sterile.destination.change")
            : t("sterile.destination.choose")}
        </button>
      </div>

      <div class="picker-row">
        <span class="picker-row__icon"><Icon name="package" size={16} /></span>
        <div class="picker-row__text">
          <p class="picker-row__label">{t("sterile.archive")}</p>
          {#if wizard.sterileArchive}
            <p class="path selectable">{wizard.sterileArchive}</p>
          {:else}
            <p class="text-muted text-sm">{t("sterile.archive.missing")}</p>
          {/if}
        </div>
        {#if wizard.sterileArchive}
          <button class="btn" onclick={clearArchive} disabled={wizard.sterileRunning}>
            {t("sterile.archive.clear")}
          </button>
        {/if}
        <button class="btn" onclick={chooseArchive} disabled={wizard.sterileRunning}>
          {wizard.sterileArchive ? t("sterile.archive.change") : t("sterile.archive.choose")}
        </button>
      </div>

      {#if wizard.sterileArchive}
        <Segmented
          label={t("archive.format")}
          options={archiveFormats}
          value={wizard.sterileArchiveFormat}
          disabled={wizard.sterileRunning}
          onselect={onFormatChange}
        />
      {/if}

      <Segmented
        label={t("sterile.safetyMode")}
        options={safeModes}
        value={wizard.sterileSafetyMode}
        disabled={wizard.sterileRunning}
        onselect={(value) => (wizard.sterileSafetyMode = value)}
      />

      <div class="launch">
        <button
          class="btn btn--primary btn--lg"
          onclick={begin}
          disabled={wizard.sterileRunning || !wizard.sterileSource || !wizard.sterileDestination}
        >
          {#if wizard.sterileRunning}
            <span class="spinner"></span>
            {t("sterile.running")}
          {:else}
            <Icon name="play" size={15} />
            {t("sterile.start")}
          {/if}
        </button>

        {#if wizard.sterileRunning}
          <button class="btn btn--danger btn--lg" onclick={stop}>
            {t("sterile.cancel")}
          </button>
          <span class="text-muted text-sm">{t("sterile.noProgress")}</span>
        {/if}
      </div>
    </div>
  </section>

  {#if wizard.sterileError}
    <Callout tone="danger" title={t("sterile.failed")}>{wizard.sterileError}</Callout>
  {/if}

  {#if wizard.sterileResult?.archive}
    {@const archive = wizard.sterileResult.archive}
    <section class="card">
      <div class="card__body archive-result">
        <span class="picker-row__icon"><Icon name="package" size={16} /></span>
        <div class="picker-row__text">
          <p class="picker-row__label">{t("sterile.archive.ready")}</p>
          <p class="path selectable">{archive.path}</p>
          <p class="text-muted text-sm">
            {archive.file_count}
            {t("sterile.archive.files")} · {formatBytes(archive.bytes)}
          </p>
        </div>
        <button class="btn" onclick={() => revealArchive(archive.path)}>
          {t("sterile.archive.reveal")}
        </button>
      </div>
    </section>
  {/if}

  {#if wizard.sterileResult}
    {@const result = wizard.sterileResult}
    <section class="card">
      <div class="card__header">
        <h2 class="card__title">{t("sterile.summary.title")}</h2>
      </div>
      <div class="card__body">
        <div class="stat-grid">
          <Stat label={t("sterile.summary.total")} value={result.summary.total_files} />
          <Stat
            tone="success"
            label={t("sterile.summary.strippedAndFormatted")}
            value={result.summary.stripped_and_formatted}
          />
          <Stat
            label={t("sterile.summary.strippedOnly")}
            value={result.summary.stripped_only_no_formatter_found}
          />
          <Stat
            label={t("sterile.summary.unsupported")}
            value={result.summary.skipped_unsupported_language}
          />
          <Stat
            tone="warning"
            label={t("sterile.summary.sensitive")}
            value={result.summary.skipped_sensitive_or_redacted}
          />
          <Stat tone="danger" label={t("sterile.summary.errors")} value={result.summary.errors} />
        </div>
      </div>
    </section>

    <section class="card card--flush">
      <div class="card__header">
        <h2 class="card__title">{t("sterile.files.title")}</h2>
      </div>
      {#if result.files.length === 0}
        <p class="log-empty">{t("sterile.files.empty")}</p>
      {:else}
        <ul class="file-list">
          {#each result.files as file (file.path)}
            <li class="file-row">
              <span class="badge badge--{outcomeTone[file.outcome]}">
                {t(outcomeLabels[file.outcome])}
              </span>
              <span class="file-row__path selectable">{file.path}</span>
              {#if detailOf(file)}
                <span class="text-muted text-sm">{detailOf(file)}</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page {
    height: 100%;
  }

  .picker-row,
  .archive-result {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    flex-wrap: wrap;
    border-radius: var(--radius-md);
  }

  /* Drop affordance for the source field only — the destination stays picker-only,
     so it never grows this state. */
  .picker-row--dragging {
    outline: 2px dashed var(--accent);
    outline-offset: 4px;
  }

  .picker-row__icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--fg-muted);
  }

  .picker-row__text {
    flex: 1;
    min-width: 180px;
  }

  .picker-row__label {
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }

  .launch {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex-wrap: wrap;
    padding-top: var(--space-3);
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: var(--space-4);
  }

  .file-list {
    display: flex;
    flex-direction: column;
    max-height: 420px;
    overflow-y: auto;
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-6);
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
  }

  .file-row__path {
    flex: 1;
    min-width: 220px;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    word-break: break-all;
  }

  .log-empty {
    padding: var(--space-9) var(--space-6);
    color: var(--fg-muted);
    font-size: var(--text-base);
    text-align: center;
  }
</style>
