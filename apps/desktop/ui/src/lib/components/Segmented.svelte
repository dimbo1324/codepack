<script lang="ts" module>
  export interface SegmentOption<T extends string> {
    value: T;
    label: string;
    /** Shown under the option list once that option is selected. A mode whose
     * consequences are invisible until export is a mode chosen blind. */
    hint?: string;
  }
</script>

<script lang="ts" generics="T extends string">
  // A picker for a small, closed set of modes. Unlike a `<select>`, every choice and its
  // consequence is on screen at once — which is what safety modes and diff modes need,
  // since the difference between them is what does and does not leave the machine.
  interface Props {
    label?: string;
    options: SegmentOption<T>[];
    value: T;
    disabled?: boolean;
    onselect: (value: T) => void;
  }

  const { label, options, value, disabled = false, onselect }: Props = $props();

  const active = $derived(options.find((option) => option.value === value));

  let track = $state<HTMLDivElement | null>(null);

  /** The radiogroup keyboard contract, which the ARIA role promises and a bare row of
   * buttons does not deliver: exactly one option is tabbable, and the arrow keys move
   * between them, wrapping at both ends. Without this, a screen reader announces "radio
   * button, 1 of 4" and then nothing responds. */
  function onKeyDown(event: KeyboardEvent, index: number): void {
    const step =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (step === 0 || disabled) return;
    event.preventDefault();
    const next = (index + step + options.length) % options.length;
    onselect(options[next].value);
    track?.querySelectorAll<HTMLButtonElement>("button")[next]?.focus();
  }
</script>

<div class="segmented">
  {#if label}<span class="segmented__label">{label}</span>{/if}
  <div class="segmented__track" role="radiogroup" aria-label={label} bind:this={track}>
    {#each options as option, index (option.value)}
      <button
        type="button"
        role="radio"
        aria-checked={option.value === value}
        tabindex={option.value === value ? 0 : -1}
        class="segmented__option"
        class:is-active={option.value === value}
        {disabled}
        onclick={() => onselect(option.value)}
        onkeydown={(event) => onKeyDown(event, index)}
      >
        {option.label}
      </button>
    {/each}
  </div>
  {#if active?.hint}
    <p class="segmented__hint">{active.hint}</p>
  {/if}
</div>

<style>
  .segmented {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .segmented__label {
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }

  .segmented__track {
    display: flex;
    gap: var(--space-1);
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    overflow-x: auto;
  }

  .segmented__option {
    flex: 1 1 0;
    min-width: max-content;
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-sm);
    color: var(--fg-secondary);
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .segmented__option:hover:not(:disabled):not(.is-active) {
    background: var(--surface-hover);
    color: var(--fg);
  }

  .segmented__option.is-active {
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    color: var(--fg);
  }

  .segmented__option:disabled {
    opacity: 0.5;
  }

  .segmented__hint {
    max-width: 62ch;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }
</style>
