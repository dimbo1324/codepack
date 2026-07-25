<script lang="ts">
  import { onMount } from "svelte";

  import { getAppInfo, loadGlobalSettings, onExportFinished, onExportProgress } from "$lib/api/client";
  import type { AppInfo } from "$lib/api/types";
  import Nav from "$lib/components/Nav.svelte";
  import { setLanguage } from "$lib/i18n/index.svelte";
  import { initTheme } from "$lib/theme/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  import AnalyticsPage from "./pages/AnalyticsPage.svelte";
  import HistoryPage from "./pages/HistoryPage.svelte";
  import LogPage from "./pages/LogPage.svelte";
  import PreviewPage from "./pages/PreviewPage.svelte";
  import ProjectPage from "./pages/ProjectPage.svelte";
  import ResultPage from "./pages/ResultPage.svelte";
  import SecurityPage from "./pages/SecurityPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";

  let appInfo = $state<AppInfo | null>(null);
  let startupError = $state<string | null>(null);

  onMount(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;

    (async () => {
      try {
        const [info, settings] = await Promise.all([getAppInfo(), loadGlobalSettings()]);
        appInfo = info;
        setLanguage(settings.language);
        initTheme(settings.theme);
      } catch (error) {
        startupError = String(error);
      }

      // Registered once, at the root, so progress keeps accumulating in the Log
      // page's buffer even if the user is looking at a different step while an
      // export they started runs in the background.
      unlistenProgress = await onExportProgress((event) => {
        if (event.run_id !== wizard.exportRunId) return;
        if (event.message) wizard.exportLog = [...wizard.exportLog, event.message];
      });
      unlistenFinished = await onExportFinished((event) => {
        if (event.run_id !== wizard.exportRunId) return;
        wizard.exportRunning = false;
        wizard.exportCancelling = false;
        wizard.exportResult = event.report;
        wizard.exportError = event.error;
        if (event.report) wizard.step = "result";
      });
    })();

    return () => {
      unlistenProgress?.();
      unlistenFinished?.();
    };
  });
</script>

<main>
  {#if startupError}
    <p class="startup-error">{startupError}</p>
  {:else if !appInfo}
    <p class="loading">Loading…</p>
  {:else}
    <Nav />
    <div class="page">
      {#if wizard.step === "project"}
        <ProjectPage {appInfo} />
      {:else if wizard.step === "settings"}
        <SettingsPage {appInfo} />
      {:else if wizard.step === "security"}
        <SecurityPage />
      {:else if wizard.step === "preview"}
        <PreviewPage />
      {:else if wizard.step === "log"}
        <LogPage />
      {:else if wizard.step === "result"}
        <ResultPage />
      {:else if wizard.step === "history"}
        <HistoryPage />
      {:else if wizard.step === "analytics"}
        <AnalyticsPage />
      {/if}
    </div>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }

  .loading,
  .startup-error {
    padding: 24px;
  }

  .startup-error {
    color: var(--danger);
  }
</style>
