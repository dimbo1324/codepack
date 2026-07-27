<script lang="ts">
  // The context strip: which project is loaded, and the two switches a user reaches for
  // most often. Both switches also write into the session config when there is one, so
  // the Settings page never shows a theme or language different from the one on screen.
  import { language, setLanguage, t } from "$lib/i18n/index.svelte";
  import { setThemePreference, theme, type ThemePreference } from "$lib/theme/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";
  import { baseName } from "$lib/util/format";

  import Icon, { type IconName } from "./Icon.svelte";

  const THEME_ORDER: ThemePreference[] = ["light", "dark", "system"];

  const themeIcon: Record<ThemePreference, IconName> = {
    light: "sun",
    dark: "moon",
    system: "monitor",
  };

  function cycleTheme(): void {
    const next = THEME_ORDER[(THEME_ORDER.indexOf(theme.preference) + 1) % THEME_ORDER.length];
    setThemePreference(next);
    if (wizard.sessionConfig) wizard.sessionConfig.theme = next;
  }

  function toggleLanguage(): void {
    const next = language.current === "ru" ? "en" : "ru";
    setLanguage(next);
    if (wizard.sessionConfig) wizard.sessionConfig.language = next;
  }
</script>

<header class="topbar">
  <div class="project">
    {#if wizard.project}
      <span class="project__icon"><Icon name="folder" size={15} /></span>
      <span class="project__name">{baseName(wizard.project.root)}</span>
      <span class="project__path truncate" title={wizard.project.root}>{wizard.project.root}</span>
    {:else}
      <span class="project__icon"><Icon name="folder" size={15} /></span>
      <span class="project__empty">{t("shell.noProject")}</span>
    {/if}
  </div>

  <div class="actions">
    {#if wizard.exportRunning}
      <span class="running">
        <span class="spinner"></span>
        {t("status.exporting")}
      </span>
    {/if}

    <button
      class="btn btn--ghost btn--icon"
      onclick={cycleTheme}
      title="{t('shell.theme')}: {t(`settings.theme.${theme.preference}`)}"
      aria-label="{t('shell.theme')}: {t(`settings.theme.${theme.preference}`)}"
    >
      <Icon name={themeIcon[theme.preference]} size={16} />
    </button>

    <button
      class="btn btn--ghost lang"
      onclick={toggleLanguage}
      title={t("shell.language")}
      aria-label={t("shell.language")}
    >
      <Icon name="globe" size={16} />
      <span>{language.current.toUpperCase()}</span>
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-6);
    height: var(--layout-header);
    flex: none;
    padding: 0 var(--space-6);
    border-bottom: 1px solid var(--border);
    background: var(--chrome);
  }

  .project {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    min-width: 0;
  }

  .project__icon {
    align-self: center;
    color: var(--fg-muted);
  }

  .project__name {
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
    white-space: nowrap;
  }

  .project__path {
    color: var(--fg-faint);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    min-width: 0;
  }

  .project__empty {
    color: var(--fg-muted);
    font-size: var(--text-base);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: none;
  }

  .running {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-right: var(--space-3);
    color: var(--accent-fg);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    white-space: nowrap;
  }

  .lang {
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.03em;
  }

  @media (max-width: 720px) {
    .project__path {
      display: none;
    }
  }
</style>
