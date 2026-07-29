<script lang="ts" module>
  export interface SegmentOption<T extends string> {
    value: T;
    label: string;
    /** Shown under the option list once that option is selected. A mode whose
     * consequences are invisible until export is a mode chosen blind. */
    hint?: string;
    /** When set, the option is offered but cannot be chosen, and this text explains
     * why. Used for a choice that genuinely exists and is not built yet: hiding it
     * would leave the user wondering whether it is coming, and letting them pick it
     * would fail later, further from the decision. Clicking shows the reason. */
    unavailable?: string;
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
  /** The explanation for the last unavailable option the user tried to pick. Cleared as
   * soon as they choose something real, so it never lingers as stale advice. */
  let blocked = $state<string | null>(null);

  function choose(option: SegmentOption<T>): void {
    if (option.unavailable) {
      blocked = option.unavailable;
      return;
    }
    blocked = null;
    onselect(option.value);
  }

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
    // Arrow keys land on an unavailable option like any other — it is focusable and
    // announced — but choosing it shows the reason instead of changing the value.
    const next = (index + step + options.length) % options.length;
    choose(options[next]);
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
        class:is-unavailable={!!option.unavailable}
        aria-disabled={!!option.unavailable}
        title={option.unavailable}
        {disabled}
        onclick={() => choose(option)}
        onkeydown={(event) => onKeyDown(event, index)}
      >
        {option.label}
      </button>
    {/each}
  </div>
  {#if blocked}
    <p class="segmented__blocked" role="status">{blocked}</p>
  {:else if active?.hint}
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

  /* Dimmed, but still readable and still focusable: the point is that the user can see
     the choice exists. */
  .segmented__option.is-unavailable {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .segmented__option.is-unavailable:hover:not(:disabled) {
    background: transparent;
    color: var(--fg-secondary);
  }

  .segmented__blocked {
    max-width: 62ch;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .segmented__hint {
    max-width: 62ch;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }
</style>
