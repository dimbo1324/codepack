<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  interface Props {
    icon?: IconName;
    title: string;
    text?: string;
    /** The action that resolves the emptiness. An empty state that only explains itself
     * leaves the user to guess where the button is. */
    action?: Snippet;
  }

  const { icon = "info", title, text, action }: Props = $props();
</script>

<div class="empty">
  <span class="empty__icon"><Icon name={icon} size={28} weight={1.4} /></span>
  <p class="empty__title">{title}</p>
  {#if text}<p class="empty__text">{text}</p>{/if}
  {#if action}<div>{@render action()}</div>{/if}
</div>
