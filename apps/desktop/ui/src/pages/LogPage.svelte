<script lang="ts">
  // The Log step is also where an export is *started*: the two belong together, because
  // pressing Export is the moment the user wants to watch what happens.
  //
  // Progress events are collected in the wizard store by `App.svelte`, not here, so the
  // buffer keeps filling while the user looks at another step.
  import { cancelExport, pickProjectDirectory, startExport } from "$lib/api/client";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  let error = $state<string | null>(null);
  let outputDir = $state<string | null>(null);
  let logElement = $state<HTMLPreElement | null>(null);

  // Follow the tail while a run is live. Reading `wizard.exportLog.length` is what makes
  // this re-run on every new line.
  $effect(() => {
    const lineCount = wizard.exportLog.length;
    if (logElement && wizard.exportRunning && lineCount > 0) {
      logElement.scrollTop = logElement.scrollHeight;
    }
  });

  async function chooseOutput(): Promise<void> {
    const chosen = await pickProjectDirectory();
    if (chosen) outputDir = chosen;
  }

  async function begin(): Promise<void> {
    if (!wizard.project || !wizard.sessionConfig || !outputDir) return;
    error = null;
    wizard.exportLog = [];
    wizard.exportResult = null;
    wizard.exportError = null;
    wizard.exportRunning = true;
    try {
      wizard.exportRunId = await startExport(
        wizard.project.root,
        outputDir,
        wizard.sessionConfig,
        wizard.fileOverrides,
      );
    } catch (err) {
      // The run never started, so no `export:finished` event will arrive to clear this.
      wizard.exportRunning = false;
      error = String(err);
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
    } catch (err) {
      wizard.exportCancelling = false;
      error = String(err);
    }
  }
</script>

<section>
  <div class="controls">
    <button onclick={chooseOutput} disabled={wizard.exportRunning}>
      {t("export.choosePath")}
    </button>
    {#if outputDir}
      <code class="path">{outputDir}</code>
    {/if}
  </div>

  <div class="controls">
    <button
      class="primary"
      onclick={begin}
      disabled={wizard.exportRunning || !outputDir || !wizard.project}
    >
      {wizard.exportRunning ? t("export.running") : t("export.start")}
    </button>
    {#if wizard.exportRunning}
      <button onclick={stop} disabled={wizard.exportCancelling}>
        {wizard.exportCancelling ? t("export.cancelling") : t("export.cancel")}
      </button>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if wizard.exportError}
    <p class="error">{wizard.exportError}</p>
  {/if}

  {#if wizard.exportLog.length === 0}
    <p class="empty">{t("log.empty")}</p>
  {:else}
    <pre bind:this={logElement}>{wizard.exportLog.join("\n")}</pre>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .path {
    font-size: 12px;
    color: var(--fg-muted);
    word-break: break-all;
  }

  .error {
    color: var(--danger);
  }

  .empty {
    color: var(--fg-muted);
    font-size: 13px;
  }

  pre {
    flex: 1;
    margin: 0;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-subtle);
    font-size: 12px;
    line-height: 1.5;
    overflow: auto;
    min-height: 240px;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
