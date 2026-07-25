<script lang="ts">
  // Past runs, from the SQLite history the engine writes on every export — including
  // cancelled and failed ones. A history that showed only successes would hide exactly
  // the runs a user needs to go back and look at.
  import { fetchHistory, openResultLocation } from "$lib/api/client";
  import type { HistoryEntry, HistoryReport } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  /** One page of history. Deliberately bounded: the table is unbounded by design, since
   * retention is opt-in through `Config.history_keep_last_n`. */
  const PAGE_SIZE = 50;

  let report = $state<HistoryReport | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let thisProjectOnly = $state(true);

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const scope = thisProjectOnly ? (wizard.project?.root ?? null) : null;
      report = await fetchHistory(scope, PAGE_SIZE);
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  // Reloads on mount and whenever the scope toggle changes; reading both dependencies
  // here is what makes that automatic.
  $effect(() => {
    void thisProjectOnly;
    void wizard.project?.root;
    void load();
  });

  /** A run's outcome, from the same fields the storage layer records. `successful` is
   * "produced a snapshot baseline" (invariant I6), not "was not cancelled" — a run that
   * failed on copy errors is not cancelled either. */
  function statusOf(entry: HistoryEntry): { key: "ok" | "cancelled" | "failed"; tone: string } {
    if (entry.cancelled) return { key: "cancelled", tone: "warning" };
    if (!entry.successful) return { key: "failed", tone: "danger" };
    return { key: "ok", tone: "ok" };
  }

  async function reveal(entry: HistoryEntry): Promise<void> {
    if (!entry.result_path) return;
    error = null;
    try {
      await openResultLocation(entry.result_path);
    } catch (err) {
      error = String(err);
    }
  }
</script>

<section>
  <div class="controls">
    <label>
      <input type="checkbox" bind:checked={thisProjectOnly} disabled={!wizard.project} />
      {thisProjectOnly ? t("history.thisProjectOnly") : t("history.allProjects")}
    </label>
    <button onclick={load} disabled={loading}>
      {loading ? t("common.loading") : t("common.retry")}
    </button>
  </div>

  {#if error}
    <p class="danger">{error}</p>
  {/if}

  {#if report && report.runs.length === 0}
    <p class="empty">{t("history.empty")}</p>
  {:else if report}
    <ul>
      {#each report.runs as entry (entry.run_id)}
        {@const status = statusOf(entry)}
        <li>
          <span class="badge {status.tone}">{t(`history.status.${status.key}`)}</span>
          <span class="when">{entry.started_at_utc}</span>
          <span class="project">{entry.project_name}</span>
          <span class="meta">
            {entry.profile ?? "—"} / {entry.safe_mode ?? "—"} / {entry.diff_mode ?? "—"}
          </span>
          {#if entry.files_copied !== null}
            <span class="meta">{t("result.filesCopied", { count: entry.files_copied })}</span>
          {/if}
          {#if entry.result_path}
            <button class="link" onclick={() => reveal(entry)}>
              {t("result.openFolder")}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 13px;
  }

  .when {
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
  }

  .project {
    font-weight: 600;
  }

  .meta {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .empty {
    color: var(--fg-muted);
    font-size: 13px;
  }

  .danger {
    color: var(--danger);
  }

  button.link {
    background: transparent;
    border-color: transparent;
    text-decoration: underline;
    padding: 0 4px;
    font-size: 12px;
  }
</style>
