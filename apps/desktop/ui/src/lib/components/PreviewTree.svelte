<script lang="ts">
  import { untrack } from "svelte";

  import type { TreeNode } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  import { formatBytes } from "$lib/util/format";
  // Self-import rather than `<svelte:self>`, which Svelte 5 deprecated.
  import PreviewTree from "./PreviewTree.svelte";
  import Icon from "./Icon.svelte";

  export interface ExpandCommand {
    /** Bumped by the parent on every expand-all/collapse-all press. Comparing the token
     * rather than the boolean is what lets the same command fire twice in a row. */
    token: number;
    expanded: boolean;
  }

  interface Props {
    node: TreeNode;
    overrides: Record<string, boolean>;
    onOverride: (path: string, include: boolean | null) => void;
    expand: ExpandCommand;
    /** Open regardless of local state. Set while a filter is active: a match hidden
     * behind a collapsed ancestor is a match the user cannot act on. Kept separate from
     * `expanded` so clearing the filter restores whatever the user opened by hand. */
    forceExpanded?: boolean;
    depth?: number;
  }

  const { node, overrides, onOverride, expand, forceExpanded = false, depth = 0 }: Props = $props();

  // Only the root level starts expanded: a deep tree opened all the way is unreadable,
  // and the interesting nodes are surfaced by the parent's aggregated status anyway.
  //
  // A node that mounts *after* an expand-all adopts that command as its starting state.
  // Without this, expand-all reached exactly one level deeper per press, because a
  // collapsed subtree has no component instances to receive the command at all.
  //
  // `untrack` states the intent explicitly. Each node renders its own component
  // instance, so neither `depth` nor the command in force at mount time changes for a
  // given instance — capturing them once is correct here, not a missed reactive
  // dependency, and without `untrack` Svelte cannot tell those two cases apart.
  let expanded = $state(untrack(() => (expand.token > 0 ? expand.expanded : depth < 1)));
  let lastToken = untrack(() => expand.token);

  $effect(() => {
    if (expand.token === lastToken) return;
    lastToken = expand.token;
    expanded = expand.expanded;
  });

  const isOpen = $derived(forceExpanded || expanded);

  const override = $derived(node.is_dir ? undefined : overrides[node.path]);
</script>

<div class="node" style:--depth={depth}>
  <div class="row" class:is-dir={node.is_dir}>
    {#if node.is_dir}
      <button
        class="twisty"
        aria-expanded={isOpen}
        aria-label={node.name}
        disabled={forceExpanded}
        onclick={() => (expanded = !expanded)}
      >
        <span class="twisty__icon" class:is-open={isOpen}><Icon name="chevron" size={12} /></span>
      </button>
    {:else}
      <span class="twisty twisty--spacer"></span>
    {/if}

    <span class="glyph status-{node.status}">
      <Icon name={node.is_dir ? "folder" : "file"} size={13} />
    </span>

    <span class="name" title={node.path}>{node.name}</span>

    {#if !node.is_dir && node.size !== null}
      <span class="size num">{formatBytes(node.size)}</span>
    {/if}

    <span class="badge badge--{node.status}">{t(`preview.status.${node.status}`)}</span>

    {#if override !== undefined}
      <span class="badge badge--accent">
        {override ? t("preview.override.forcedIn") : t("preview.override.forcedOut")}
      </span>
    {/if}

    {#if node.reason}
      <span class="reason truncate" title={node.reason}>{node.reason}</span>
    {/if}

    {#if !node.is_dir}
      <span class="actions">
        <button
          class="btn btn--sm btn--ghost"
          title={t("preview.override.include")}
          aria-label={t("preview.override.include")}
          onclick={() => onOverride(node.path, true)}
        >
          <Icon name="plus" size={12} />
        </button>
        <button
          class="btn btn--sm btn--ghost"
          title={t("preview.override.exclude")}
          aria-label={t("preview.override.exclude")}
          onclick={() => onOverride(node.path, false)}
        >
          <Icon name="minus" size={12} />
        </button>
        {#if override !== undefined}
          <button
            class="btn btn--sm btn--ghost"
            title={t("preview.override.reset")}
            aria-label={t("preview.override.reset")}
            onclick={() => onOverride(node.path, null)}
          >
            <Icon name="refresh" size={12} />
          </button>
        {/if}
      </span>
    {/if}
  </div>

  {#if node.is_dir && isOpen && node.children}
    {#each node.children as child (child.path)}
      <PreviewTree
        node={child}
        {overrides}
        {onOverride}
        {expand}
        {forceExpanded}
        depth={depth + 1}
      />
    {/each}
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 2px var(--space-5) 2px calc(var(--space-4) + var(--depth) * 14px);
    border-radius: var(--radius-xs);
    font-size: var(--text-sm);
    min-height: 24px;
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .twisty {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    flex: none;
    border-radius: var(--radius-xs);
    color: var(--fg-muted);
  }

  .twisty:hover {
    background: var(--surface-active);
    color: var(--fg);
  }

  .twisty--spacer {
    pointer-events: none;
  }

  .twisty__icon {
    display: block;
    transition: transform var(--duration-fast) var(--ease-out);
  }

  .twisty__icon.is-open {
    transform: rotate(90deg);
  }

  .glyph {
    flex: none;
    color: var(--fg-faint);
  }

  .glyph.status-included {
    color: var(--success);
  }

  .glyph.status-warning {
    color: var(--warning);
  }

  .name {
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .is-dir .name {
    font-weight: var(--weight-medium);
  }

  .size {
    color: var(--fg-faint);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .reason {
    flex: 1;
    min-width: 0;
    color: var(--fg-muted);
    font-size: var(--text-xs);
  }

  /* Kept out of the layout until the row is hovered or something inside it has focus:
     three buttons on every one of ten thousand rows is the clutter, not the density. */
  .actions {
    display: flex;
    gap: var(--space-1);
    margin-left: auto;
    padding-left: var(--space-4);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .row:hover .actions,
  .actions:focus-within {
    opacity: 1;
  }
</style>
