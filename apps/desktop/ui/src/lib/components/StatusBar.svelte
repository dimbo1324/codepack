<script lang="ts">
  // The privacy statement lives here, permanently visible, rather than as one line on the
  // first page that scrolls away and is never seen again. It is the product's central
  // promise (invariant I1), and a promise worth making is worth keeping on screen.
  import type { AppInfo } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";

  import Icon from "./Icon.svelte";

  interface Props {
    appInfo: AppInfo;
  }

  const { appInfo }: Props = $props();
</script>

<footer class="statusbar">
  <span class="item item--privacy" title={appInfo.network_access}>
    <Icon name="lock" size={12} />
    {t("status.local")}
  </span>

  <span class="spacer"></span>

  {#if wizard.watchActive}
    <span class="item item--watch">
      <Icon name="eye" size={12} />
      {t("status.watchOn")}
      {#if wizard.watchChangedPaths.length > 0}
        <span class="badge badge--warning">{wizard.watchChangedPaths.length}</span>
      {/if}
    </span>
  {/if}

  <span class="item">{t("status.version", { version: appInfo.version })}</span>
</footer>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    height: var(--layout-statusbar);
    flex: none;
    padding: 0 var(--space-6);
    border-top: 1px solid var(--border);
    background: var(--chrome);
    color: var(--fg-muted);
    font-size: var(--text-xs);
    white-space: nowrap;
    overflow: hidden;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item--watch {
    color: var(--fg-secondary);
  }

  @media (max-width: 720px) {
    .item--privacy {
      display: none;
    }
  }
</style>
