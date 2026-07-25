<script lang="ts">
  import { scanProject } from "$lib/api/client";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  let error = $state<string | null>(null);

  async function runScan(): Promise<void> {
    if (!wizard.project || !wizard.sessionConfig) return;
    error = null;
    wizard.scanLoading = true;
    try {
      wizard.scan = await scanProject(wizard.project.root, wizard.sessionConfig);
    } catch (err) {
      error = String(err);
    } finally {
      wizard.scanLoading = false;
    }
  }
</script>

{#if wizard.sessionConfig}
  {@const config = wizard.sessionConfig}
  <section>
    <label>
      {t("security.safeMode")}
      <select bind:value={config.safe_export_mode}>
        <option value="safe">{t("security.safeMode.safe")}</option>
        <option value="balanced">{t("security.safeMode.balanced")}</option>
        <option value="full">{t("security.safeMode.full")}</option>
      </select>
    </label>

    <button class="primary" onclick={runScan} disabled={wizard.scanLoading}>
      {wizard.scanLoading ? t("security.scanning") : t("security.scan")}
    </button>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if wizard.scan}
      <div class="report">
        {#if wizard.scan.findings.length === 0}
          <p>{t("security.noFindings")}</p>
        {:else}
          <p>
            {t("security.findings", { count: wizard.scan.summary.total_findings })}
            {#if wizard.scan.summary.critical > 0}
              <span class="badge critical">
                {t("security.critical", { count: wizard.scan.summary.critical })}
              </span>
            {/if}
          </p>
          <ul>
            {#each wizard.scan.findings as finding (finding.file + (finding.line ?? "") + finding.rule)}
              <li>
                <span class="badge {finding.severity === 'critical' ? 'critical' : 'warning'}">
                  {finding.severity}
                </span>
                <code>{finding.file}{finding.line ? `:${finding.line}` : ""}</code>
                — {finding.message}
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 640px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .error {
    color: var(--danger);
  }

  ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
  }
</style>
