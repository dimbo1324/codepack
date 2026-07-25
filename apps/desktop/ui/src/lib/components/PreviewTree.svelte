<script lang="ts">
  import { untrack } from "svelte";

  import type { TreeNode } from "$lib/api/types";
  import { t } from "$lib/i18n/index.svelte";
  // Self-import rather than `<svelte:self>`, which Svelte 5 deprecated.
  import PreviewTree from "./PreviewTree.svelte";

  interface Props {
    node: TreeNode;
    overrides: Record<string, boolean>;
    onOverride: (path: string, include: boolean | null) => void;
    depth?: number;
  }

  const { node, overrides, onOverride, depth = 0 }: Props = $props();

  // Only the root level starts expanded: a deep tree opened all the way is unreadable,
  // and the interesting nodes are surfaced by the parent's aggregated status anyway.
  //
  // `untrack` states the intent explicitly. Each node renders its own component
  // instance, so `depth` never changes for a given instance — capturing it once is
  // correct here, not a missed reactive dependency, and without `untrack` Svelte cannot
  // tell those two cases apart.
  let expanded = $state(untrack(() => depth) < 1);

  const override = $derived(node.is_dir ? undefined : overrides[node.path]);
</script>

<div class="node" style:padding-left="{depth * 16}px">
  <div class="row">
    {#if node.is_dir}
      <button class="twisty" onclick={() => (expanded = !expanded)}>
        {expanded ? "▾" : "▸"}
      </button>
    {:else}
      <span class="twisty-spacer"></span>
    {/if}

    <span class="name" title={node.path}>{node.name}</span>

    <span class="badge {node.status}">
      {t(`preview.status.${node.status}`)}
    </span>

    {#if override !== undefined}
      <span class="badge" title="Manual override">
        {override ? "→ include" : "→ exclude"}
      </span>
    {/if}

    {#if node.reason}
      <span class="reason">{node.reason}</span>
    {/if}

    {#if !node.is_dir}
      <span class="actions">
        <button onclick={() => onOverride(node.path, true)}>
          {t("preview.override.include")}
        </button>
        <button onclick={() => onOverride(node.path, false)}>
          {t("preview.override.exclude")}
        </button>
        {#if override !== undefined}
          <button onclick={() => onOverride(node.path, null)}>
            {t("preview.override.reset")}
          </button>
        {/if}
      </span>
    {/if}
  </div>

  {#if node.is_dir && expanded && node.children}
    {#each node.children as child (child.path)}
      <PreviewTree node={child} {overrides} {onOverride} depth={depth + 1} />
    {/each}
  {/if}
</div>

<style>
  .node {
    font-size: 13px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
  }

  .twisty {
    background: transparent;
    border: none;
    padding: 0 4px;
    width: 20px;
  }

  .twisty-spacer {
    display: inline-block;
    width: 20px;
  }

  .name {
    font-family: ui-monospace, monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .reason {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .actions {
    margin-left: auto;
    display: flex;
    gap: 4px;
  }

  .actions button {
    padding: 2px 6px;
    font-size: 12px;
  }
</style>
