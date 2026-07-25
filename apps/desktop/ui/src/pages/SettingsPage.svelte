<script lang="ts">
  import { applyPreset, applyProfile, saveGlobalSettings } from "$lib/api/client";
  import type { AppInfo } from "$lib/api/types";
  import { setLanguage, t } from "$lib/i18n/index.svelte";
  import { setThemePreference } from "$lib/theme/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  interface Props {
    appInfo: AppInfo;
  }
  const { appInfo }: Props = $props();

  let selectedPreset = $state("");
  let saveStatus = $state<string | null>(null);
  let applyError = $state<string | null>(null);

  async function onPresetChange(): Promise<void> {
    if (!wizard.sessionConfig || !selectedPreset) return;
    applyError = null;
    try {
      wizard.sessionConfig = await applyPreset(wizard.sessionConfig, selectedPreset);
    } catch (error) {
      applyError = String(error);
    }
  }

  async function onProfileChange(name: string): Promise<void> {
    if (!wizard.sessionConfig || !name) return;
    applyError = null;
    try {
      wizard.sessionConfig = await applyProfile(wizard.sessionConfig, name);
    } catch (error) {
      applyError = String(error);
    }
  }

  async function saveAsDefaults(): Promise<void> {
    if (!wizard.sessionConfig) return;
    await saveGlobalSettings(wizard.sessionConfig);
    saveStatus = t("settings.saved");
    setTimeout(() => (saveStatus = null), 2000);
  }
</script>

{#if wizard.sessionConfig}
  {@const config = wizard.sessionConfig}
  <section class="grid">
    <label>
      {t("settings.preset")}
      <select bind:value={selectedPreset} onchange={onPresetChange}>
        <option value="">{t("settings.preset.none")}</option>
        {#each appInfo.presets as preset (preset.name)}
          <option value={preset.name}>{preset.name} — {preset.description}</option>
        {/each}
      </select>
    </label>

    <label>
      {t("settings.profile")}
      <select
        value={config.export_profile}
        onchange={(event) => onProfileChange(event.currentTarget.value)}
      >
        {#each appInfo.profiles as profile (profile.key)}
          <option value={profile.key}>{profile.label}</option>
        {/each}
      </select>
    </label>

    <label>
      {t("settings.diffMode")}
      <select bind:value={config.diff_export_mode}>
        <option value="all">all</option>
        <option value="last_export">last_export</option>
        <option value="git_ref">git_ref</option>
        <option value="uncommitted">uncommitted</option>
      </select>
    </label>

    <label class="checkbox">
      <input type="checkbox" bind:checked={config.redact_secrets} />
      {t("settings.redactSecrets")}
    </label>

    <label class="checkbox">
      <input type="checkbox" bind:checked={config.include_git_patch} />
      {t("settings.includeGitPatch")}
    </label>

    <label class="checkbox">
      <input type="checkbox" bind:checked={config.keep_staging_folder} />
      {t("settings.keepStaging")}
    </label>

    <label>
      {t("settings.tokenBudget")}
      <input type="number" min="0" bind:value={config.token_budget} />
    </label>

    <label>
      {t("settings.developerContext")}
      <textarea rows="2" bind:value={config.developer_context}></textarea>
    </label>

    <label class="checkbox">
      <input type="checkbox" bind:checked={config.watch_enabled} />
      {t("settings.watch")}
    </label>

    <label class="checkbox">
      <input type="checkbox" bind:checked={config.watch_clipboard_auto_update} />
      {t("settings.watchClipboard")}
    </label>

    <hr />

    <label>
      {t("settings.theme")}
      <select
        bind:value={config.theme}
        onchange={() => setThemePreference(config.theme)}
      >
        <option value="light">{t("settings.theme.light")}</option>
        <option value="dark">{t("settings.theme.dark")}</option>
        <option value="system">{t("settings.theme.system")}</option>
      </select>
    </label>

    <label>
      {t("settings.language")}
      <select
        bind:value={config.language}
        onchange={() => setLanguage(config.language)}
      >
        <option value="en">English</option>
        <option value="ru">Русский</option>
      </select>
    </label>

    <label>
      {t("settings.uiZoom")}
      <input type="range" min="0.75" max="2" step="0.05" bind:value={config.ui_zoom} />
      {Math.round(config.ui_zoom * 100)}%
    </label>

    <div class="actions">
      <button onclick={saveAsDefaults}>{t("settings.save")}</button>
      {#if saveStatus}<span class="status">{saveStatus}</span>{/if}
    </div>

    {#if applyError}
      <p class="error">{applyError}</p>
    {/if}
  </section>
{/if}

<style>
  .grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 480px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  label.checkbox {
    flex-direction: row;
    align-items: center;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status {
    color: var(--success);
  }

  .error {
    color: var(--danger);
  }
</style>
