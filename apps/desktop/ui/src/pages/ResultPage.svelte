<script lang="ts">
  // What the export produced, and the two things a user wants next: find the bundle, or
  // read its reports.
  //
  // A run that did not finish cleanly still lands here with a bundle — the pipeline
  // always writes a manifest and always archives what it collected. Saying so plainly
  // matters more than hiding it: an incomplete bundle that looks complete is the one
  // outcome a tool about safe handoff must never produce.
  import { openDashboard, openResultLocation } from "$lib/api/client";
  import { t } from "$lib/i18n/index.svelte";
  import { goTo, wizard } from "$lib/stores/wizard.svelte";

  let error = $state<string | null>(null);

  async function reveal(path: string): Promise<void> {
    error = null;
    try {
      await openResultLocation(path);
    } catch (err) {
      error = String(err);
    }
  }

  async function showDashboard(path: string): Promise<void> {
    error = null;
    try {
      await openDashboard(path);
    } catch (err) {
      error = String(err);
    }
  }
</script>

{#if wizard.exportResult}
  {@const result = wizard.exportResult}
  <section>
    <h2>
      {result.cancelled ? t("export.cancelled") : t("result.title")}
    </h2>

    {#if !result.successful}
      <p class="warning">{t("result.incomplete")}</p>
    {/if}

    <ul class="stats">
      <li>{t("result.filesCopied", { count: result.files_copied })}</li>
      <li>{t("result.filesSkipped", { count: result.files_skipped })}</li>
      {#if result.errors > 0}
        <li class="danger">{t("result.errors", { count: result.errors })}</li>
      {/if}
      {#if result.critical_findings > 0}
        <li class="danger">
          {t("result.criticalFindings", { count: result.critical_findings })}
        </li>
      {/if}
    </ul>

    {#if result.archives.length > 0}
      <ul class="archives">
        {#each result.archives as archive (archive)}
          <li><code>{archive}</code></li>
        {/each}
      </ul>
    {/if}

    <div class="actions">
      {#if result.result_path}
        {@const path = result.result_path}
        <button class="primary" onclick={() => reveal(path)}>
          {t("result.openFolder")}
        </button>
        <button onclick={() => showDashboard(path)}>
          {t("result.openDashboard")}
        </button>
      {/if}
      <button onclick={() => goTo("history")}>{t("result.viewHistory")}</button>
    </div>

    {#if error}
      <p class="danger">{error}</p>
    {/if}
  </section>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 720px;
  }

  h2 {
    margin: 0;
    font-size: 18px;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .stats {
    flex-direction: row;
    flex-wrap: wrap;
    gap: 16px;
  }

  .archives code {
    font-size: 12px;
    word-break: break-all;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .warning {
    color: var(--warning);
  }

  .danger {
    color: var(--danger);
  }
</style>
