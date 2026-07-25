<script lang="ts">
  import type { TranslationKey } from "$lib/i18n/en";
  import { t } from "$lib/i18n/index.svelte";
  import { goTo, STEPS, wizard, type Step } from "$lib/stores/wizard.svelte";

  const labels: Record<Step, TranslationKey> = {
    project: "nav.project",
    settings: "nav.settings",
    security: "nav.security",
    preview: "nav.preview",
    log: "nav.log",
    result: "nav.result",
    history: "nav.history",
    analytics: "nav.analytics",
  };

  // Every step past "project" needs a project selected; "result"/"analytics" are
  // only meaningful once an export has actually produced something.
  function isReachable(step: Step): boolean {
    if (step === "project") return true;
    if (!wizard.project) return false;
    if ((step === "result" || step === "analytics") && !wizard.exportResult) return false;
    return true;
  }
</script>

<nav>
  {#each STEPS as step (step)}
    <button
      class:active={wizard.step === step}
      disabled={!isReachable(step)}
      onclick={() => goTo(step)}
    >
      {t(labels[step])}
    </button>
  {/each}
</nav>

<style>
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  button {
    background: transparent;
    border-color: transparent;
  }

  button.active {
    background: var(--bg);
    border-color: var(--border);
    font-weight: 600;
  }
</style>
