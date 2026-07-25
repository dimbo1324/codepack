<script lang="ts">
  import { previewProject } from "$lib/api/client";
  import PreviewTree from "$lib/components/PreviewTree.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  let error = $state<string | null>(null);

  async function runPreview(): Promise<void> {
    if (!wizard.project || !wizard.sessionConfig) return;
    error = null;
    wizard.previewLoading = true;
    try {
      wizard.preview = await previewProject(
        wizard.project.root,
        wizard.sessionConfig,
        wizard.fileOverrides,
      );
    } catch (err) {
      error = String(err);
    } finally {
      wizard.previewLoading = false;
    }
  }

  function onOverride(path: string, include: boolean | null): void {
    const next = { ...wizard.fileOverrides };
    if (include === null) {
      delete next[path];
    } else {
      next[path] = include;
    }
    wizard.fileOverrides = next;
  }
</script>

<section>
  <button class="primary" onclick={runPreview} disabled={wizard.previewLoading}>
    {wizard.previewLoading ? t("preview.loading") : t("preview.run")}
  </button>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if wizard.preview}
    {@const preview = wizard.preview}
    <p class="summary">
      {t("preview.summary", {
        included: preview.included_files,
        excluded: preview.excluded_files,
        size: preview.estimated_bytes_human,
        tokens: preview.estimated_tokens,
      })}
    </p>
    {#if preview.dropped_by_budget > 0}
      <p class="note">{t("preview.budgetDropped", { count: preview.dropped_by_budget })}</p>
    {/if}
    {#if preview.sensitive_count > 0}
      <p class="note warning">{t("preview.sensitive", { count: preview.sensitive_count })}</p>
    {/if}

    <div class="tree">
      <PreviewTree node={preview.tree} overrides={wizard.fileOverrides} {onOverride} />
    </div>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .error {
    color: var(--danger);
  }

  .note {
    color: var(--fg-muted);
    font-size: 13px;
  }

  .note.warning {
    color: var(--warning);
  }

  .tree {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px;
    overflow: auto;
    max-height: 60vh;
  }
</style>
