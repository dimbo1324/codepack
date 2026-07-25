<script lang="ts">
  // The project passport an export wrote, read back from the bundle rather than
  // recomputed. Recomputing would answer a subtly different question — "what does the
  // project look like now" instead of "what did you actually hand over" — and the second
  // is the one this page exists for.
  import { readProjectProfile } from "$lib/api/client";
  import type { ProjectProfileSummary } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  let summary = $state<ProjectProfileSummary | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  $effect(() => {
    const path = wizard.exportResult?.result_path;
    if (!path) {
      summary = null;
      return;
    }
    loading = true;
    error = null;
    readProjectProfile(path)
      .then((loaded) => {
        summary = loaded;
      })
      .catch((err: unknown) => {
        error = String(err);
      })
      .finally(() => {
        loading = false;
      });
  });

  /** Binary units, matching `codepack_tokens::format_bytes` so the GUI and the reports
   * quote the same number the same way. */
  function formatBytes(size: number): string {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = size;
    for (let index = 0; index < units.length; index += 1) {
      if (value < 1024 || index === units.length - 1) {
        return index === 0 ? `${Math.trunc(value)} ${units[index]}` : `${value.toFixed(2)} ${units[index]}`;
      }
      value /= 1024;
    }
    return `${size} B`;
  }

  function riskTone(level: string): string {
    if (level === "high" || level === "critical") return "danger";
    if (level === "medium") return "warning";
    return "ok";
  }
</script>

<section>
  <h2>{t("analytics.title")}</h2>

  {#if loading}
    <p class="muted">{t("common.loading")}</p>
  {:else if error}
    <p class="danger">{error}</p>
  {:else if !summary}
    <p class="muted">{t("analytics.none")}</p>
  {:else}
    {@const profile = summary}
    <dl>
      <dt>{t("analytics.projectType")}</dt>
      <dd>{profile.project_type || "—"}</dd>

      <dt>{t("analytics.stack")}</dt>
      <dd>{profile.detected_stack.length > 0 ? profile.detected_stack.join(", ") : "—"}</dd>

      <dt>{t("analytics.riskLevel")}</dt>
      <dd>
        <span class="badge {riskTone(profile.risk_level)}">{profile.risk_level || "—"}</span>
      </dd>

      {#if profile.risk_reasons.length > 0}
        <dt>{t("analytics.riskReasons")}</dt>
        <dd>
          <ul>
            {#each profile.risk_reasons as reason (reason)}
              <li>{reason}</li>
            {/each}
          </ul>
        </dd>
      {/if}

      <dt>{t("nav.analytics")}</dt>
      <dd>
        {t("analytics.counts", {
          files: profile.files,
          folders: profile.folders,
          size: formatBytes(profile.total_size_bytes),
        })}
      </dd>
    </dl>
  {/if}
</section>

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

  dl {
    display: grid;
    grid-template-columns: minmax(120px, auto) 1fr;
    gap: 8px 16px;
    margin: 0;
    font-size: 13px;
  }

  dt {
    color: var(--fg-muted);
  }

  dd {
    margin: 0;
  }

  dd ul {
    list-style: disc;
    margin: 0;
    padding-left: 18px;
  }

  .muted {
    color: var(--fg-muted);
    font-size: 13px;
  }

  .danger {
    color: var(--danger);
  }
</style>
