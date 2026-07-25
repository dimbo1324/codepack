<script lang="ts">
  import { openProject, pickProjectDirectory } from "$lib/api/client";
  import type { AppInfo } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { resetForNewProject, wizard } from "$lib/stores/wizard.svelte";

  interface Props {
    appInfo: AppInfo;
  }
  const { appInfo }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function choose(): Promise<void> {
    error = null;
    const path = await pickProjectDirectory();
    if (!path) return;

    busy = true;
    try {
      const context = await openProject(path);
      resetForNewProject(context);
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <h1>{t("app.title")}</h1>
  <p class="privacy">{appInfo.network_access}</p>

  {#if wizard.project}
    <p>
      <strong>{t("project.chosen")}:</strong>
      <code>{wizard.project.root}</code>
    </p>
    <p class="muted">
      {#if wizard.project.resolution.project_config}
        {t("project.configSource.found")} <code>{appInfo.project_config_file_name}</code>
      {:else}
        {t("project.configSource.none")}
      {/if}
    </p>
  {:else}
    <p class="muted">{t("project.none")}</p>
  {/if}

  <button class="primary" onclick={choose} disabled={busy}>
    {busy ? t("common.loading") : t("project.choose")}
  </button>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</section>

<style>
  .privacy {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .muted {
    color: var(--fg-muted);
  }

  .error {
    color: var(--danger);
  }
</style>
