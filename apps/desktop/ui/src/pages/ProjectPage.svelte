<script lang="ts">
  import { pickProjectDirectory } from "$lib/api/client";
  import type { AppInfo } from "$lib/api/types";
  import { openProjectAt, projectAction } from "$lib/actions/project.svelte";
  import Callout from "$lib/components/Callout.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { pushToast } from "$lib/stores/toasts.svelte";
  import { goTo, wizard } from "$lib/stores/wizard.svelte";
  import { copyText } from "$lib/util/clipboard";
  import { baseName } from "$lib/util/format";

  interface Props {
    appInfo: AppInfo;
  }
  const { appInfo }: Props = $props();

  async function choose(): Promise<void> {
    const path = await pickProjectDirectory();
    if (path) await openProjectAt(path);
  }

  async function copyRoot(): Promise<void> {
    if (!wizard.project) return;
    const copied = await copyText(wizard.project.root);
    pushToast(copied ? "success" : "danger", copied ? "common.copied" : "common.copyFailed");
  }
</script>

<div class="stack">
  <div class="page-header">
    <div>
      <h1 class="page-title">{t("project.title")}</h1>
      <p class="page-lede">{t("project.lede")}</p>
    </div>
  </div>

  {#if wizard.project}
    {@const project = wizard.project}
    <div class="card">
      <div class="card__body">
        <div class="chosen">
          <span class="chosen__icon"><Icon name="folder-open" size={20} weight={1.5} /></span>
          <div class="chosen__text">
            <p class="chosen__name">{baseName(project.root)}</p>
            <p class="path selectable">{project.root}</p>
          </div>
          <div class="row row--tight">
            <button class="btn btn--sm" onclick={copyRoot}>
              <Icon name="copy" size={13} />
              {t("common.copy")}
            </button>
            <button class="btn btn--sm" onclick={choose} disabled={projectAction.opening}>
              {projectAction.opening ? t("project.opening") : t("project.change")}
            </button>
          </div>
        </div>

        <dl class="kv resolution">
          <dt>{t("project.configSource.found")}</dt>
          <dd>
            {#if project.resolution.project_config}
              <code class="selectable">{appInfo.project_config_file_name}</code>
            {:else}
              <span class="text-muted">{t("project.configSource.none")}</span>
            {/if}
          </dd>

          <dt>{t("project.resolution.preset")}</dt>
          <dd>{project.resolution.preset ?? t("common.none")}</dd>

          <dt>{t("project.resolution.profile")}</dt>
          <dd>{project.resolution.profile ?? t("common.none")}</dd>
        </dl>
      </div>
      <div class="card__header footer">
        <span class="text-muted text-sm">{t("project.resolution")}</span>
        <button class="btn btn--primary" onclick={() => goTo("settings")}>
          {t("project.continue")}
          <Icon name="chevron" size={14} />
        </button>
      </div>
    </div>
  {:else}
    <div class="card">
      <div class="card__body">
        <div class="picker">
          <span class="picker__icon"><Icon name="folder-open" size={34} weight={1.3} /></span>
          <p class="picker__title">{t("project.none")}</p>
          <p class="picker__text">{t("project.noneText")}</p>
          <button
            class="btn btn--primary btn--lg"
            onclick={choose}
            disabled={projectAction.opening}
          >
            {#if projectAction.opening}
              <span class="spinner"></span>
              {t("project.opening")}
            {:else}
              <Icon name="folder" size={16} />
              {t("project.choose")}
            {/if}
          </button>
          <p class="picker__drop">{t("project.drop")}</p>
        </div>
      </div>
    </div>
  {/if}

  <Callout tone="info" icon="lock">{appInfo.network_access}</Callout>
</div>

<style>
  .chosen {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    flex-wrap: wrap;
  }

  .chosen__icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    color: var(--accent-fg);
  }

  .chosen__text {
    flex: 1;
    min-width: 200px;
  }

  .chosen__name {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }

  .resolution {
    margin-top: var(--space-7);
    padding-top: var(--space-6);
    border-top: 1px solid var(--border);
  }

  .footer {
    border-top: 1px solid var(--border);
    border-bottom: 0;
  }

  .picker {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-9) var(--space-6);
    text-align: center;
  }

  .picker__icon {
    color: var(--fg-faint);
  }

  .picker__title {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }

  .picker__text {
    max-width: 46ch;
    color: var(--fg-muted);
    line-height: var(--leading-relaxed);
  }

  .picker__drop {
    margin-top: var(--space-2);
    color: var(--fg-faint);
    font-size: var(--text-sm);
  }
</style>
