<script lang="ts">
  // A checkbox that reads as a setting rather than as a form field: the label is the hit
  // target, the explanation sits under it, and the real `<input type="checkbox">` stays
  // in the DOM (visually hidden) so keyboard, screen readers and form semantics are the
  // browser's job, not ours.
  interface Props {
    label: string;
    hint?: string;
    checked: boolean;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  const { label, hint, checked, disabled = false, onchange }: Props = $props();
</script>

<label class="switch" class:switch--disabled={disabled}>
  <input
    type="checkbox"
    {checked}
    {disabled}
    onchange={(event) => onchange(event.currentTarget.checked)}
  />
  <span class="switch__track" aria-hidden="true"><span class="switch__thumb"></span></span>
  <span class="switch__text">
    <span class="switch__label">{label}</span>
    {#if hint}<span class="switch__hint">{hint}</span>{/if}
  </span>
</label>

<style>
  .switch {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    gap: var(--space-2) var(--space-5);
  }

  .switch--disabled {
    opacity: 0.5;
  }

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    margin: 0;
  }

  .switch__track {
    position: relative;
    width: 34px;
    height: 20px;
    margin-top: 1px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-sunken);
    transition:
      background var(--duration-base) var(--ease-out),
      border-color var(--duration-base) var(--ease-out);
  }

  .switch__thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--fg-muted);
    transition:
      transform var(--duration-base) var(--ease-out),
      background var(--duration-base) var(--ease-out);
  }

  input:checked + .switch__track {
    background: var(--accent);
    border-color: transparent;
  }

  input:checked + .switch__track .switch__thumb {
    background: #fff;
    transform: translateX(14px);
  }

  input:focus-visible + .switch__track {
    outline: 2px solid var(--border-focus);
    outline-offset: 2px;
  }

  .switch__text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .switch__label {
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }

  .switch__hint {
    max-width: 62ch;
    color: var(--fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }
</style>
