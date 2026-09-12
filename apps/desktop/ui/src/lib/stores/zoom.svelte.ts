// The window's zoom factor, and the four ways it changes.
//
// Zoom existed before this store: `Config::ui_zoom`, a slider in Settings, and
// `set_ui_zoom` to apply it. What it did not have was any way to reach from the window
// itself, which is why the owner's report was that the interface was large and crowded
// with nothing to do about it.
//
// One store owns the current factor so the four routes — the keyboard, the wheel, the
// status bar and the Settings slider — cannot disagree about what the zoom is, and so the
// readout has something to read.
//
// ## Applying and persisting are separate calls on purpose
//
// `setUiZoom` changes the webview and nothing else; `saveUiZoom` writes the factor to the
// settings file and turns `ui_zoom_auto` off. Startup applies without persisting — the
// derived factor is not a decision the user made, and writing it would silently convert
// "follow my monitor" into "stay at 87% forever", including after they plug in a bigger
// screen.

import { monitorZoom, saveUiZoom, setUiZoom, startupZoom } from "$lib/api/client";

/** The range `Config::ui_zoom` is clamped to, restated from `codepack_core::config`. The
 *  backend clamps again, so this is for the interface's own arithmetic — a disabled
 *  button at the end of the range rather than one that silently does nothing. */
export const ZOOM_MIN = 0.7;
export const ZOOM_MAX = 1.5;

/** The grid the keyboard and the status bar step along.
 *
 *  Stepping snaps to multiples of this rather than adding to whatever is current, so a
 *  monitor-derived 0.87 steps to a round 0.9 or 0.85 instead of 0.92 or 0.82. The readout
 *  shows whole percentages either way, and a readout that goes 87 → 92 → 97 looks like a
 *  bug even when it is not. */
const STEP = 0.05;

class ZoomState {
  /** What the webview is at. */
  current = $state(1);
  /** Whether this factor came from the monitor rather than from the user. Mirrors
   *  `Config::ui_zoom_auto`, and goes false the moment the user changes anything. */
  auto = $state(true);
}

export const zoom = new ZoomState();

/** Rounds to two decimals, matching what the backend stores, so the readout and the
 *  settings file never disagree by a floating-point hair. */
function round(factor: number): number {
  return Math.round(factor * 100) / 100;
}

function clamp(factor: number): number {
  if (!Number.isFinite(factor)) return 1;
  return round(Math.min(Math.max(factor, ZOOM_MIN), ZOOM_MAX));
}

/** The next multiple of `STEP` in the given direction, so stepping lands on round values
 *  even when the starting point is a derived one.
 *
 *  **In integer hundredths, not in floats.** `factor / STEP` is exact for most values and
 *  not for all: `1.2 / 0.05` is `23.999999999999996`, so `Math.floor(…) + 1` lands back on
 *  24 and stepping up returns the number it started from. Found by running the app —
 *  zooming out worked and zooming in silently did nothing — and it affected six values in
 *  the permitted range, including 0.70, which is exactly where somebody on a small screen
 *  would be sitting when they pressed the key. */
function stepFrom(factor: number, direction: 1 | -1): number {
  const cents = Math.round(factor * 100);
  const stepCents = Math.round(STEP * 100);
  const grid =
    direction === 1 ? Math.floor(cents / stepCents) + 1 : Math.ceil(cents / stepCents) - 1;
  return clamp((grid * stepCents) / 100);
}

/** Applies a factor to the window, and remembers it as the user's choice.
 *
 *  Every user-initiated route goes through here. The apply is awaited before the state
 *  updates so the readout cannot claim a zoom the window refused. */
export async function setZoom(factor: number): Promise<void> {
  const next = clamp(factor);
  await setUiZoom(next);
  zoom.current = next;
  zoom.auto = false;
  // Persisted separately and not awaited by the caller's rendering: the window has
  // already changed, and a settings file that cannot be written should not make the zoom
  // appear not to work.
  void saveUiZoom(next).catch(() => undefined);
}

export function zoomIn(): Promise<void> {
  return setZoom(stepFrom(zoom.current, 1));
}

export function zoomOut(): Promise<void> {
  return setZoom(stepFrom(zoom.current, -1));
}

/** `Ctrl 0`: back to whatever this monitor suggests.
 *
 *  Not "back to 100%", which is the browser convention, because 100% is what the owner
 *  reported as too large on a 150%-scaled display. Returning to the derived factor is
 *  both the reset and the adaptation, and it is the only route that turns `auto` back
 *  on. */
export async function resetZoom(): Promise<void> {
  // `monitorZoom`, not `startupZoom`: the latter honours `ui_zoom_auto`, so once the user
  // has chosen a factor it returns that stored choice — and this route would have been
  // unable to get back to the monitor's own suggestion, which is the only thing it exists
  // to do. Found while verifying the shortcuts against the running app.
  const derived = clamp(await monitorZoom());
  await setUiZoom(derived);
  zoom.current = derived;
  zoom.auto = true;
  void saveUiZoom(derived, true).catch(() => undefined);
}

/** Applies the zoom this launch should open at, without recording it as a choice. */
export async function initZoom(): Promise<void> {
  const factor = clamp(await startupZoom());
  await setUiZoom(factor);
  zoom.current = factor;
}

export const canZoomIn = () => zoom.current < ZOOM_MAX;
export const canZoomOut = () => zoom.current > ZOOM_MIN;

/** Whether a keyboard event should be treated as a zoom shortcut.
 *
 *  Excludes text entry: `Ctrl -` inside a text field is not a zoom request, and a user
 *  typing into the question box on the result page would otherwise resize the window
 *  under themselves. */
export function zoomShortcut(event: KeyboardEvent): "in" | "out" | "reset" | null {
  if (!event.ctrlKey && !event.metaKey) return null;
  if (event.altKey) return null;

  const target = event.target as HTMLElement | null;
  if (target?.isContentEditable) return null;
  const tag = target?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return null;

  switch (event.key) {
    // `=` and `+` are the same physical key; which one arrives depends on Shift and on
    // the keyboard layout, and `Add`/`Subtract` are the numeric keypad's names.
    case "+":
    case "=":
    case "Add":
      return "in";
    case "-":
    case "_":
    case "Subtract":
      return "out";
    case "0":
      return "reset";
    default:
      return null;
  }
}
