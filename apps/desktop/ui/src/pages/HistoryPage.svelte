<script lang="ts">
  // Past runs, from the SQLite history the engine writes on every export — including
  // cancelled and failed ones. A history that showed only successes would hide exactly
  // the runs a user needs to go back and look at.
  import { fetchHistory, openResultLocation } from "$lib/api/client";
  import type { AppInfo, HistoryEntry, HistoryReport } from "$lib/api/types";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Segmented, { type SegmentOption } from "$lib/components/Segmented.svelte";
  import { diffModeLabel, safeModeLabel } from "$lib/i18n/enums";
  import { language, t } from "$lib/i18n/index.svelte";
  import { reportError } from "$lib/stores/toasts.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";
  import { formatBytes, formatCount, formatRelativeTime } from "$lib/util/format";

  interface Props {
    appInfo: AppInfo;
  }
  const { appInfo }: Props = $props();

  /** One page of history. Deliberately bounded: the table is unbounded by design, since
   * retention is opt-in through `Config.history_keep_last_n`. */
  const PAGE_SIZE = 50;

  /** The backend already owns the profile display names; a second table here would
   * eventually disagree with it. An unknown key falls through as itself — a run recorded
   * by an older build must still show what it actually used. */
  function profileLabel(key: string | null): string {
    if (!key) return t("common.none");
    return appInfo.profiles.find((profile) => profile.key === key)?.label ?? key;
  }

  let report = $state<HistoryReport | null>(null);
  let loading = $state(false);
  let scope = $state<"project" | "all">("project");

  const scopes: SegmentOption<"project" | "all">[] = $derived([
    { value: "project", label: t("history.thisProjectOnly") },
    { value: "all", label: t("history.allProjects") },
  ]);

  async function load(): Promise<void> {
    loading = true;
    try {
      const root = scope === "project" ? (wizard.project?.root ?? null) : null;
      report = await fetchHistory(root, PAGE_SIZE);
    } catch (error) {
      reportError("history.loadFailed", error);
    } finally {
      loading = false;
    }
  }

  // Reloads on mount and whenever the scope toggle changes; reading both dependencies
  // here is what makes that automatic.
  $effect(() => {
    void scope;
    void wizard.project?.root;
    void load();
  });

  /** A run's outcome, from the same fields the storage layer records. `successful` is
   * "produced a snapshot baseline" (invariant I6), not "was not cancelled" — a run that
   * failed on copy errors is not cancelled either. */
  function statusOf(entry: HistoryEntry): { key: "ok" | "cancelled" | "failed"; tone: string } {
    if (entry.cancelled) return { key: "cancelled", tone: "warning" };
    if (!entry.successful) return { key: "failed", tone: "critical" };
    return { key: "ok", tone: "ok" };
  }

  async function reveal(entry: HistoryEntry): Promise<void> {
    if (!entry.result_path) return;
    try {
      await openResultLocation(entry.result_path);
    } catch (error) {
      reportError("result.openFailed", error);
    }
  }
</script>

<div class="stack">
  <div class="page-header">
    <div>
      <h1 class="page-title">{t("history.title")}</h1>
      <p class="page-lede">{t("history.lede")}</p>
    </div>
    <div class="row row--tight">
      <Segmented
        options={scopes}
        value={scope}
        disabled={!wizard.project}
        onselect={(value) => (scope = value)}
      />
      <button class="btn" onclick={load} disabled={loading}>
        {#if loading}<span class="spinner"></span>{:else}<Icon name="refresh" size={14} />{/if}
        {t("history.refresh")}
      </button>
    </div>
  </div>

  <section class="card card--flush">
    {#if report && report.runs.length === 0}
      <EmptyState icon="clock" title={t("history.empty")} text={t("history.emptyText")} />
    {:else if report}
      <div class="scroll">
        <table class="table">
          <thead>
            <tr>
              <th>{t("history.column.status")}</th>
              <th>{t("history.column.when")}</th>
              <th>{t("history.column.project")}</th>
              <th>{t("history.column.settings")}</th>
              <th class="num">{t("history.column.files")}</th>
              <th class="num">{t("history.column.size")}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each report.runs as entry (entry.run_id)}
              {@const status = statusOf(entry)}
              <tr>
                <td>
                  <span class="badge badge--{status.tone}">
                    {t(`history.status.${status.key}`)}
                  </span>
                </td>
                <td class="when" title={entry.started_at_utc}>
                  {formatRelativeTime(entry.started_at, language.current)}
                </td>
                <td>
                  <span class="project" title={entry.project_root}>{entry.project_name}</span>
                </td>
                <td class="settings">
                  {profileLabel(entry.profile)} ·
                  {entry.safe_mode ? safeModeLabel(entry.safe_mode) : t("common.none")} ·
                  {entry.diff_mode ? diffModeLabel(entry.diff_mode) : t("common.none")}
                </td>
                <td class="num">
                  {entry.files_copied === null
                    ? t("common.none")
                    : formatCount(entry.files_copied, language.current)}
                </td>
                <td class="num">
                  {entry.bytes_total === null ? t("common.none") : formatBytes(entry.bytes_total)}
                </td>
                <td class="actions">
                  {#if entry.result_path}
                    <button
                      class="btn btn--sm btn--ghost"
                      title={t("result.openFolder")}
                      aria-label={t("result.openFolder")}
                      onclick={() => reveal(entry)}
                    >
                      <Icon name="folder-open" size={14} />
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <div class="loading"><span class="spinner"></span></div>
    {/if}
  </section>
</div>

<style>
  .scroll {
    max-height: 68vh;
    overflow: auto;
  }

  .when {
    color: var(--fg-muted);
    white-space: nowrap;
  }

  .project {
    font-weight: var(--weight-medium);
  }

  .settings {
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }

  .actions {
    width: 1%;
    white-space: nowrap;
  }

  .loading {
    display: grid;
    place-items: center;
    padding: var(--space-10);
  }
</style>
