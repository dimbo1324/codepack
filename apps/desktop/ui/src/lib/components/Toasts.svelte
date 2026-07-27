<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import { dismissToast, toasts } from "$lib/stores/toasts.svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  const icons: Record<string, IconName> = {
    info: "info",
    success: "check-circle",
    warning: "alert",
    danger: "alert",
  };
</script>

<!-- `polite`, not `assertive`: these announce completed work, and interrupting whatever
     the user is reading to say "saved" is worse than saying it a moment later. -->
<div class="toasts" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <div class="toast toast--{toast.tone}">
      <span class="toast__icon"><Icon name={icons[toast.tone]} size={16} /></span>
      <div class="toast__body">
        <p class="toast__title">{t(toast.titleKey, toast.params)}</p>
        {#if toast.detail}<p class="toast__detail selectable">{toast.detail}</p>{/if}
      </div>
      <button
        class="btn btn--ghost btn--icon btn--sm"
        aria-label={t("common.dismiss")}
        onclick={() => dismissToast(toast.id)}
      >
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: var(--space-6);
    bottom: calc(var(--layout-statusbar) + var(--space-6));
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: min(400px, calc(100vw - var(--space-9)));
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-4) var(--space-4) var(--space-5);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: var(--shadow-lg);
    pointer-events: auto;
    animation: toast-in var(--duration-base) var(--ease-out);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }

  .toast__icon {
    margin-top: 2px;
    color: var(--fg-muted);
  }

  .toast--success .toast__icon {
    color: var(--success);
  }

  .toast--warning .toast__icon {
    color: var(--warning);
  }

  .toast--danger {
    border-color: var(--danger-soft-border);
  }

  .toast--danger .toast__icon {
    color: var(--danger);
  }

  .toast__body {
    flex: 1;
    min-width: 0;
  }

  .toast__title {
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }

  .toast__detail {
    margin-top: var(--space-2);
    max-height: 6.5em;
    overflow: auto;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    word-break: break-word;
  }
</style>
