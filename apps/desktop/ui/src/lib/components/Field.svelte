<script lang="ts">
  // Label + explanation + control, as one element.
  //
  // The wrapper is the `<label>` itself rather than a `<div>` holding one: implicit
  // association needs no generated ids, and it cannot drift the way a `for`/`id` pair
  // does when a control is moved between pages.
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    /** Why this setting exists, in the user's terms. Most controls here change what
     * leaves the machine, so an unexplained toggle is a real hazard, not a style issue. */
    hint?: string;
    children: Snippet;
  }

  const { label, hint, children }: Props = $props();
</script>

<label class="field">
  <span class="field__label">{label}</span>
  {#if hint}
    <span class="field__hint">{hint}</span>
  {/if}
  <span class="field__control">{@render children()}</span>
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field__label {
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }

  .field__hint {
    max-width: 62ch;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .field__control {
    display: block;
    margin-top: var(--space-1);
  }
</style>
