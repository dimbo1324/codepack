<script lang="ts">
  // The Export step is also where a run is *started*: pressing Export is the moment the
  // user wants to watch what happens, so the controls, the pipeline state and the log all
  // belong on one page.
  //
  // Progress events are collected in the wizard store by `App.svelte`, not here, so the
  // buffer keeps filling while the user looks at another step.
  import { cancelExport, pickProjectDirectory, startExport } from "$lib/api/client";
  import Callout from "$lib/components/Callout.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import StepProgress from "$lib/components/StepProgress.svelte";
  import { language, t } from "$lib/i18n/index.svelte";
  import { pushToast, reportError } from "$lib/stores/toasts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { adoptExportRun, goTo, resetExportState, wizard } from "$lib/stores/wizard.svelte";
  import { copyText } from "$lib/util/clipboard";
  import { formatCount, formatDuration } from "$lib/util/format";

  let logElement = $state<HTMLPreElement | null>(null);
  let now = $state(Date.now());

  // One ticker for the elapsed clock, alive only while a run is. A timer that keeps
  // running after the export finished would redraw the page forever for nothing.
  $effect(() => {
    if (!wizard.exportRunning) return;
    const handle = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(handle);
  });

  const elapsed = $derived.by(() => {
    if (wizard.exportStartedAt === null) return null;
    const end = wizard.exportRunning ? now : (wizard.exportFinishedAt ?? now);
    return formatDuration(end - wizard.exportStartedAt);
  });

  // Follow the tail while a run is live. Reading `wizard.exportLog.length` is what makes
  // this re-run on every new line; the user can switch it off to read back through it.
  $effect(() => {
    const lineCount = wizard.exportLog.length;
    if (logElement && ui.logFollow && lineCount > 0) {
      logElement.scrollTop = logElement.scrollHeight;
    }
  });

  async function chooseOutput(): Promise<void> {
    const chosen = await pickProjectDirectory();
    if (chosen) wizard.outputDir = chosen;
  }

  async function begin(): Promise<void> {
    if (!wizard.project || !wizard.sessionConfig || !wizard.outputDir) return;
    resetExportState();
    wizard.exportRunning = true;
    wizard.exportStartedAt = Date.now();
    now = Date.now();
    try {
      // The engine starts emitting before this promise resolves; `adoptExportRun`
      // replays whatever arrived in that window rather than dropping it.
      adoptExportRun(
        await startExport(
          wizard.project.root,
          wizard.outputDir,
          $state.snapshot(wizard.sessionConfig),
          { ...wizard.fileOverrides },
        ),
      );
    } catch (error) {
      // The run never started, so no `export:finished` event will arrive to clear this.
      wizard.exportRunning = false;
      wizard.exportStartedAt = null;
      reportError("export.startFailed", error);
    }
  }

  async function stop(): Promise<void> {
    if (!wizard.exportRunId) return;
    // The button switches to "cancelling" immediately, but the run stays live until the
    // pipeline reaches its next cancellation check — steps 7 and 8 still complete, and
    // the bundle still describes what it managed to collect.
    wizard.exportCancelling = true;
    try {
      await cancelExport(wizard.exportRunId);
    } catch (error) {
      wizard.exportCancelling = false;
      reportError("export.cancelFailed", error);
    }
  }

  async function copyLog(): Promise<void> {
    const copied = await copyText(wizard.exportLog.join("\n"));
    pushToast(copied ? "success" : "danger", copied ? "log.copied" : "log.copyFailed");
  }
</script>

<div class="stack page">
  <div class="page-header">
    <div>
      <h1 class="page-title">{t("export.title")}</h1>
      <p class="page-lede">{t("export.lede")}</p>
    </div>
  </div>

  <section class="card">
    <div class="card__body stack--tight stack">
      <div class="output">
        <span class="output__icon"><Icon name="download" size={16} /></span>
        <div class="output__text">
          <p class="output__label">{t("export.output")}</p>
          {#if wizard.outputDir}
            <p class="path selectable">{wizard.outputDir}</p>
          {:else}
            <p class="text-muted text-sm">{t("export.outputMissing")}</p>
          {/if}
        </div>
        <button class="btn" onclick={chooseOutput} disabled={wizard.exportRunning}>
          {wizard.outputDir ? t("export.changePath") : t("export.choosePath")}
        </button>
      </div>

      <div class="launch">
        <button
          class="btn btn--primary btn--lg"
          onclick={begin}
          disabled={wizard.exportRunning || !wizard.outputDir || !wizard.project}
        >
          {#if wizard.exportRunning}
            <span class="spinner"></span>
            {t("export.running")}
          {:else}
            <Icon name="play" size={15} />
            {t("export.start")}
          {/if}
        </button>

        {#if wizard.exportRunning}
          <button class="btn btn--danger btn--lg" onclick={stop} disabled={wizard.exportCancelling}>
            {wizard.exportCancelling ? t("export.cancelling") : t("export.cancel")}
          </button>
        {/if}

        {#if elapsed}
          <span class="elapsed">
            <Icon name="clock" size={13} />
            {t("export.elapsed")}
            <span class="num">{elapsed}</span>
          </span>
        {/if}

        {#if wizard.exportRunning}
          <span class="elapsed" aria-live="polite">
            {t("export.progress", {
              current: wizard.exportStepCurrent ?? wizard.exportStepsDone,
              total: wizard.exportStepTotal,
            })}
          </span>
        {/if}

        <span class="spacer"></span>

        {#if wizard.exportResult}
          <button class="btn" onclick={() => goTo("result")}>
            {t("export.viewResult")}
            <Icon name="chevron" size={14} />
          </button>
        {/if}
      </div>
    </div>

    {#if wizard.exportStartedAt !== null}
      <div class="card__body progress">
        <StepProgress
          done={wizard.exportStepsDone}
          current={wizard.exportStepCurrent}
          total={wizard.exportStepTotal}
          running={wizard.exportRunning}
        />
      </div>
    {/if}
  </section>

  {#if wizard.exportError}
    <Callout tone="danger" title={t("export.failed")}>{wizard.exportError}</Callout>
  {/if}

  <section class="card card--flush log-card">
    <div class="card__header">
      <div>
        <h2 class="card__title">{t("log.title")}</h2>
        <p class="card__subtitle">
          {t("log.lines", { count: formatCount(wizard.exportLog.length, language.current) })}
          {#if wizard.exportLogDropped > 0}
            · {t("log.dropped", {
              count: formatCount(wizard.exportLogDropped, language.current),
            })}
          {/if}
        </p>
      </div>
      <div class="row row--tight">
        <label class="follow">
          <input type="checkbox" bind:checked={ui.logFollow} />
          {t("log.autoscroll")}
        </label>
        <button
          class="btn btn--sm"
          onclick={copyLog}
          disabled={wizard.exportLog.length === 0}
          title={t("log.copy")}
        >
          <Icon name="copy" size={13} />
          {t("common.copy")}
        </button>
      </div>
    </div>

    {#if wizard.exportLog.length === 0}
      <p class="log-empty">{t("log.empty")}</p>
    {:else}
      <pre bind:this={logElement} class="log selectable">{wizard.exportLog.join("\n")}</pre>
    {/if}
  </section>
</div>

<style>
  .page {
    height: 100%;
  }

  .output {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    flex-wrap: wrap;
  }

  .output__icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--fg-muted);
  }

  .output__text {
    flex: 1;
    min-width: 180px;
  }

  .output__label {
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

  .elapsed {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--fg-muted);
    font-size: var(--text-sm);
  }

  .progress {
    border-top: 1px solid var(--border);
  }

  .log-card {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 260px;
  }

  .follow {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--fg-secondary);
    font-size: var(--text-sm);
  }

  .follow input {
    accent-color: var(--accent);
  }

  .log {
    flex: 1;
    margin: 0;
    padding: var(--space-5) var(--space-6);
    background: var(--surface-sunken);
    color: var(--fg-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .log-empty {
    padding: var(--space-9) var(--space-6);
    color: var(--fg-muted);
    font-size: var(--text-base);
    text-align: center;
  }
</style>
