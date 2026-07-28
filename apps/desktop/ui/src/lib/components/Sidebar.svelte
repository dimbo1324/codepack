<script lang="ts">
  // The primary navigation: the export journey as a numbered sequence, and the three
  // views that report on it as a separate group.
  //
  // A step the user cannot open yet stays clickable and explains itself. The previous
  // version disabled the button, which removed it from the tab order and left the user
  // to guess which earlier step they had missed.
  import type { TranslationKey } from "$lib/i18n/en";
  import { t } from "$lib/i18n/index.svelte";
  import { pushToast } from "$lib/stores/toasts.svelte";
  import { toggleSidebar, ui } from "$lib/stores/ui.svelte";
  import {
    blockedReason,
    goTo,
    INSIGHT_STEPS,
    STANDALONE_STEPS,
    wizard,
    WIZARD_STEPS,
    type Step,
  } from "$lib/stores/wizard.svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  const labels: Record<Step, TranslationKey> = {
    project: "nav.project",
    settings: "nav.settings",
    security: "nav.security",
    preview: "nav.preview",
    export: "nav.export",
    result: "nav.result",
    history: "nav.history",
    analytics: "nav.analytics",
    sterile: "nav.sterile",
  };

  const icons: Record<Step, IconName> = {
    project: "folder",
    settings: "sliders",
    security: "shield",
    preview: "tree",
    export: "play",
    result: "package",
    history: "clock",
    analytics: "chart",
    sterile: "file",
  };

  /** A workflow step counts as visited once the user has moved past it, which is the
   * honest claim available here: the wizard has no per-step "confirmed" action, so
   * anything stronger would be a guess dressed up as a checkmark. */
  function isVisited(step: Step): boolean {
    const position = WIZARD_STEPS.indexOf(step);
    const current = WIZARD_STEPS.indexOf(wizard.step);
    return position !== -1 && current !== -1 && position < current;
  }

  function open(step: Step): void {
    const blocked = blockedReason(step);
    if (blocked) {
      pushToast("info", `nav.locked.${blocked}`);
      return;
    }
    goTo(step);
  }
</script>

<aside class="sidebar" class:is-collapsed={ui.sidebarCollapsed}>
  <div class="brand">
    <span class="brand__mark" aria-hidden="true"><Icon name="package" size={18} /></span>
    <span class="brand__text">
      <span class="brand__name">{t("app.title")}</span>
      <span class="brand__tagline">{t("app.tagline")}</span>
    </span>
  </div>

  <nav class="nav" aria-label={t("nav.section.workflow")}>
    <p class="nav__heading">{t("nav.section.workflow")}</p>
    <ul>
      {#each WIZARD_STEPS as step, index (step)}
        {@const blocked = blockedReason(step)}
        <li>
          <button
            class="nav__item"
            class:is-active={wizard.step === step}
            class:is-blocked={blocked !== null}
            aria-current={wizard.step === step ? "page" : undefined}
            aria-disabled={blocked !== null}
            title={blocked ? `${t(labels[step])} — ${t(`nav.locked.${blocked}`)}` : t(labels[step])}
            onclick={() => open(step)}
          >
            <span class="nav__marker" aria-hidden="true">
              {#if blocked}
                <Icon name="lock" size={12} />
              {:else if isVisited(step)}
                <Icon name="check" size={12} weight={2.4} />
              {:else}
                {index + 1}
              {/if}
            </span>
            <span class="nav__icon" aria-hidden="true"><Icon name={icons[step]} size={16} /></span>
            <span class="nav__label">{t(labels[step])}</span>
          </button>
        </li>
      {/each}
    </ul>

    <p class="nav__heading">{t("nav.section.insights")}</p>
    <ul>
      {#each INSIGHT_STEPS as step (step)}
        {@const blocked = blockedReason(step)}
        <li>
          <button
            class="nav__item nav__item--plain"
            class:is-active={wizard.step === step}
            class:is-blocked={blocked !== null}
            aria-current={wizard.step === step ? "page" : undefined}
            aria-disabled={blocked !== null}
            title={blocked ? `${t(labels[step])} — ${t(`nav.locked.${blocked}`)}` : t(labels[step])}
            onclick={() => open(step)}
          >
            <span class="nav__icon" aria-hidden="true"><Icon name={icons[step]} size={16} /></span>
            <span class="nav__label">{t(labels[step])}</span>
          </button>
        </li>
      {/each}
    </ul>

    <p class="nav__heading">{t("nav.section.tools")}</p>
    <ul>
      {#each STANDALONE_STEPS as step (step)}
        {@const blocked = blockedReason(step)}
        <li>
          <button
            class="nav__item nav__item--plain"
            class:is-active={wizard.step === step}
            class:is-blocked={blocked !== null}
            aria-current={wizard.step === step ? "page" : undefined}
            aria-disabled={blocked !== null}
            title={blocked ? `${t(labels[step])} — ${t(`nav.locked.${blocked}`)}` : t(labels[step])}
            onclick={() => open(step)}
          >
            <span class="nav__icon" aria-hidden="true"><Icon name={icons[step]} size={16} /></span>
            <span class="nav__label">{t(labels[step])}</span>
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <button
    class="collapse"
    onclick={toggleSidebar}
    aria-label={ui.sidebarCollapsed ? t("nav.expand") : t("nav.collapse")}
    title={ui.sidebarCollapsed ? t("nav.expand") : t("nav.collapse")}
  >
    <span class="collapse__icon" class:is-flipped={!ui.sidebarCollapsed}>
      <Icon name="chevron" size={14} />
    </span>
  </button>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: var(--layout-sidebar);
    flex: none;
    background: var(--chrome);
    border-right: 1px solid var(--border);
    overflow: hidden;
    transition: width var(--duration-base) var(--ease-out);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    height: var(--layout-header);
    padding: 0 var(--space-5);
    flex: none;
  }

  .brand__mark {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--fg-on-accent);
  }

  .brand__text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .brand__name {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
    line-height: 1.2;
  }

  .brand__tagline {
    color: var(--fg-muted);
    font-size: var(--text-2xs);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nav {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-4) var(--space-6);
  }

  .nav__heading {
    padding: var(--space-5) var(--space-4) var(--space-3);
    color: var(--fg-faint);
    font-size: var(--text-2xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .nav__item {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-sm);
    color: var(--fg-secondary);
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
    text-align: left;
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .nav__item:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

  .nav__item.is-active {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }

  .nav__item.is-blocked {
    color: var(--fg-faint);
  }

  .nav__marker {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    flex: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    color: var(--fg-muted);
    font-size: var(--text-2xs);
    font-variant-numeric: tabular-nums;
  }

  .nav__item.is-active .nav__marker {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--fg-on-accent);
  }

  .nav__icon {
    flex: none;
    opacity: 0.85;
  }

  .nav__label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nav__item--plain {
    padding-left: calc(var(--space-4) + 18px + var(--space-4));
  }

  .collapse {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    flex: none;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
  }

  .collapse:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

  .collapse__icon {
    display: block;
    transition: transform var(--duration-base) var(--ease-out);
  }

  .collapse__icon.is-flipped {
    transform: rotate(180deg);
  }

  /* --- Rail: the user's own collapse, and the width where the label no longer fits. */
  .sidebar.is-collapsed {
    width: var(--layout-rail);
  }

  .sidebar.is-collapsed .brand {
    justify-content: center;
    padding: 0;
  }

  .sidebar.is-collapsed .brand__text,
  .sidebar.is-collapsed .nav__label,
  .sidebar.is-collapsed .nav__marker {
    display: none;
  }

  .sidebar.is-collapsed .nav__heading {
    padding-inline: 0;
    text-align: center;
    overflow: hidden;
  }

  .sidebar.is-collapsed .nav__item,
  .sidebar.is-collapsed .nav__item--plain {
    justify-content: center;
    padding: var(--space-4);
  }

  @media (max-width: 1000px) {
    .sidebar {
      width: var(--layout-rail);
    }

    .sidebar .brand {
      justify-content: center;
      padding: 0;
    }

    .sidebar .brand__text,
    .sidebar .nav__label,
    .sidebar .nav__marker {
      display: none;
    }

    .sidebar .nav__heading {
      padding-inline: 0;
      text-align: center;
      overflow: hidden;
    }

    .sidebar .nav__item,
    .sidebar .nav__item--plain {
      justify-content: center;
      padding: var(--space-4);
    }

    .collapse {
      display: none;
    }
  }

  /* Below this the rail costs more than it gives: the navigation moves to a scrollable
     strip across the top, and the content keeps the full width. */
  @media (max-width: 720px) {
    .sidebar {
      width: 100%;
      flex-direction: row;
      align-items: center;
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }

    .brand {
      padding: 0 var(--space-4);
    }

    .nav {
      display: flex;
      align-items: center;
      gap: var(--space-2);
      padding: var(--space-3);
      overflow-x: auto;
      overflow-y: hidden;
    }

    .nav__heading {
      display: none;
    }

    .nav ul {
      display: flex;
      gap: var(--space-2);
    }
  }
</style>
