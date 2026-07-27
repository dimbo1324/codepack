<script lang="ts" module>
  // A hand-drawn 24x24 stroke icon set, inlined as path data.
  //
  // Inline rather than an icon package or a sprite file for two reasons that both matter
  // here: the CSP admits no remote source and no `data:` script, and an icon font or SVG
  // library would be a production dependency carrying hundreds of glyphs for the two
  // dozen this app uses. Every path below is geometry, so it inherits `currentColor` and
  // needs no light/dark variant.
  export const ICONS = {
    folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z",
    "folder-open": "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v1H3Z M3 10h18l-2 8H5Z",
    sliders: "M4 7h9 M17 7h3 M4 17h3 M11 17h9 M15 4v6 M7 14v6",
    shield: "M12 3l8 3v6c0 4.5-3.2 8.3-8 9.5C7.2 20.3 4 16.5 4 12V6Z",
    tree: "M4 6h6 M4 12h4 M4 18h10 M14 6h6 M12 12h8 M18 18h2",
    play: "M7 5l12 7-12 7Z",
    check: "M4 12.5l5 5L20 6.5",
    "check-circle": "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z M8 12l3 3 5-5",
    clock: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z M12 7v5l3 2",
    chart: "M4 20V4 M4 20h16 M8 16v-5 M13 16V8 M18 16v-3",
    sun: "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z M12 2v2 M12 20v2 M4.9 4.9l1.4 1.4 M17.7 17.7l1.4 1.4 M2 12h2 M20 12h2 M4.9 19.1l1.4-1.4 M17.7 6.3l1.4-1.4",
    moon: "M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z",
    monitor: "M3 5h18v11H3Z M9 20h6 M12 16v4",
    globe:
      "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z M3 12h18 M12 3c2.5 2.6 3.8 5.7 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.7-3.8-9S9.5 5.6 12 3Z",
    chevron: "M9 6l6 6-6 6",
    "chevron-down": "M6 9l6 6 6-6",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Z M16.5 16.5L21 21",
    x: "M6 6l12 12 M18 6L6 18",
    copy: "M9 9h10v10H9Z M15 9V5H5v10h4",
    external: "M14 4h6v6 M20 4l-8 8 M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
    refresh: "M20 12a8 8 0 1 1-2.6-5.9 M20 4v4h-4",
    alert: "M12 4l9 16H3Z M12 10v4 M12 17.5v.5",
    info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z M12 11v5 M12 7.5v.5",
    lock: "M5 11h14v9H5Z M8 11V8a4 4 0 0 1 8 0v3",
    file: "M6 3h7l5 5v13H6Z M13 3v5h5",
    eye: "M2 12s3.6-6 10-6 10 6 10 6-3.6 6-10 6-10-6-10-6Z M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z",
    "eye-off":
      "M4 4l16 16 M9.5 9.6A3 3 0 0 0 12 15c.8 0 1.6-.3 2.2-.9 M6.3 6.6C3.7 8.2 2 12 2 12s3.6 6 10 6c1.9 0 3.5-.5 4.9-1.3 M10 6.2A9.9 9.9 0 0 1 12 6c6.4 0 10 6 10 6a18 18 0 0 1-3 3.5",
    plus: "M12 5v14 M5 12h14",
    minus: "M5 12h14",
    package: "M12 3l8 4.5v9L12 21l-8-4.5v-9Z M4 7.5l8 4.5 8-4.5 M12 12v9",
    activity: "M3 12h4l3 8 4-16 3 8h4",
    filter: "M3 5h18l-7 8v6l-4 2v-8Z",
    download: "M12 4v11 M8 11l4 4 4-4 M4 19h16",
  } as const;

  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
    /** Stroke width in the icon's own 24-unit space, so it stays optically even as
     * `size` changes rather than thinning out at small sizes. */
    weight?: number;
  }

  const { name, size = 16, weight = 1.7 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={weight}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  <path d={ICONS[name]} />
</svg>

<style>
  svg {
    flex: none;
  }
</style>
