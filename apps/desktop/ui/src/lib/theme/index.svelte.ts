// Light / dark / system theme (`docs/__arch__/ROADMAP.md` §3), synchronized with `Config.theme`.
// Applied as a `data-theme` attribute on `<html>`; `src/app.css` reacts to it. Kept
// independent of any CSS framework so it costs nothing beyond a handful of variables.
export type ThemePreference = "light" | "dark" | "system";

class ThemeState {
  preference = $state<ThemePreference>("system");
  /** The theme actually applied right now — resolves "system" to what the OS
   * reports, so components that need to know (e.g. an icon) do not each re-query
   * `matchMedia` themselves. */
  resolved = $state<"light" | "dark">("light");
}

export const theme = new ThemeState();

let media: MediaQueryList | null = null;

function applyResolved(): void {
  const value =
    theme.preference === "system" ? (media?.matches ? "dark" : "light") : theme.preference;
  theme.resolved = value;
  document.documentElement.dataset.theme = value;
}

/** Call once, at startup. Wires the system-preference listener so a "system" theme
 * follows the OS live, without the user having to reopen the app. */
export function initTheme(preference: ThemePreference): void {
  theme.preference = preference;
  media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", applyResolved);
  applyResolved();
}

export function setThemePreference(preference: ThemePreference): void {
  theme.preference = preference;
  applyResolved();
}
