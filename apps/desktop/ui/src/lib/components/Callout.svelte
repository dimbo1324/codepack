<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  type Tone = "neutral" | "info" | "success" | "warning" | "danger";

  interface Props {
    tone?: Tone;
    title?: string;
    icon?: IconName;
    children: Snippet;
  }

  const { tone = "neutral", title, icon, children }: Props = $props();

  const defaultIcon: Record<Tone, IconName> = {
    neutral: "info",
    info: "info",
    success: "check-circle",
    warning: "alert",
    danger: "alert",
  };
</script>

<div class="callout callout--{tone}" role={tone === "danger" ? "alert" : undefined}>
  <span class="callout__icon"><Icon name={icon ?? defaultIcon[tone]} size={16} /></span>
  <div class="callout__body">
    {#if title}<p class="callout__title">{title}</p>{/if}
    <div class="selectable">{@render children()}</div>
  </div>
</div>
