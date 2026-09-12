<script lang="ts">
  // The privacy statement lives here, permanently visible, rather than as one line on the
  // first page that scrolls away and is never seen again. It is the product's central
  // promise (invariant I1), and a promise worth making is worth keeping on screen.
  import type { AppInfo } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { wizard } from "$lib/stores/wizard.svelte";
  import { canZoomIn, canZoomOut, resetZoom, zoom, zoomIn, zoomOut } from "$lib/stores/zoom.svelte";

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

  <!-- The zoom control lives here rather than only in Settings because the owner's
       report was that the interface was too large with nothing to do about it: a
       setting you have to go looking for is a setting nobody finds. The keyboard
       shortcuts do the same job for people who already know them. -->
  <span class="item item--zoom">
    <button
      class="zoomctl"
      onclick={() => void zoomOut().catch(() => undefined)}
      disabled={!canZoomOut()}
      title={t("status.zoom.out")}
      aria-label={t("status.zoom.out")}
    >
      <Icon name="minus" size={12} />
    </button>
    <button
      class="zoomctl zoomctl--value num"
      onclick={() => void resetZoom().catch(() => undefined)}
      title={zoom.auto ? t("status.zoom.auto") : t("status.zoom.reset")}
    >
      {Math.round(zoom.current * 100)}%{zoom.auto ? " " + t("status.zoom.autoMark") : ""}
    </button>
    <button
      class="zoomctl"
      onclick={() => void zoomIn().catch(() => undefined)}
      disabled={!canZoomIn()}
      title={t("status.zoom.in")}
      aria-label={t("status.zoom.in")}
    >
      <Icon name="plus" size={12} />
    </button>
  </span>

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

  .item--zoom {
    gap: var(--space-1);
  }

  .zoomctl {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 var(--space-2);
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .zoomctl:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--border);
  }

  .zoomctl:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .zoomctl--value {
    min-width: 44px;
    font-variant-numeric: tabular-nums;
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
