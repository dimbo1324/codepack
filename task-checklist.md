# Task Checklist

**Task:** Make the desktop window's zoom changeable from the window itself, and make the
application fit whatever monitor it opens on. Owner's words: "сейчас всё большое и
нагромождённое".

**Date:** 2026-09-12
**Branch:** `feat/desktop-zoom-and-monitor-fit`

Branched off `feat/s13-api-path-command-and-screen` rather than `main`, deliberately: that
branch is finished and gate-green but not merged, and it edits `SettingsPage.svelte` and
`ResultPage.svelte`, which this task edits too. Building on top avoids a conflict nobody
gains from. The rules permit building on an unmerged branch and forbid only rewriting one.

## What is actually wrong — measured, not guessed

The owner's monitor is **1920×1200 physical**, and Windows reports the desktop as
**1280×800 logical** — display scaling is **150%**. The working area, minus the taskbar,
is **1280×752**.

Against that, three things are wrong at once:

1. **The default window is 1100×760** (`tauri.conf.json`), and 760 is *taller than the 752
   available*. The window does not fit the screen it opens on, out of the box, before
   anything else is considered.
2. **The layout is designed for more width than exists.** `--layout-sidebar: 232px` plus
   `--layout-content-max: 1080px` wants 1312 logical px; the screen has 1280 and the
   window is given 1100. The sidebar collapses to a rail only below 1000px, so at this
   size the full sidebar stays and the content is squeezed throughout. That is the
   "crowded".
3. **Everything renders at 1.5×.** A 13px font is 19.5 physical pixels. That is the
   "large" — and it is the operating system's scaling, not a font choice, which is why
   the type scale being modest on paper does not help.

Zoom already exists (`Config::ui_zoom`, `set_ui_zoom`, a Settings control, range 0.7–1.5)
and **already solves most of this**: at 0.7 the CSS viewport becomes roughly 1828×1074,
which is roomy. What is missing is that nothing tells the user it exists, there is no
keyboard route to it, and the default is 1.0 regardless of the monitor. So this task is
largely about *reaching* a mechanism that is already there — plus making the window fit.

## What this task will not do

- **No redesign and no new type scale.** The px-absolute token sizes are a recorded
  decision with a stated reason: a second relative layer would compound with the native
  webview zoom. Zoom is the right lever, and adding a second is how two levers start
  fighting each other.
- **No change to the 0.7–1.5 range.** 0.7 is already ample on this monitor. Widening a
  normalisation bound deserves its own decision, and this task does not need it.

## Step 0 — preparation

- [+] Orientation: the previous task's checklist is complete, and its CI run went on
      to pass all three OS legs
- [+] Measure the real environment rather than guess (done above)
- [+] Branch, and this checklist committed **before** the work (`d013c97`)

## Step 1 — the window fits the monitor it opens on

- [+] At startup the shell reads the current monitor and shrinks the window to fit,
      treating the configured size as a maximum rather than a demand.
      `Monitor::work_area` exists in Tauri 2.11, so no taskbar is guessed at
- [+] Re-centred inside the work area afterwards, not inside the monitor, so it cannot
      land under a taskbar along an edge
- [+] A monitor smaller than `minWidth`/`minHeight` does not produce an unusable window:
      the minimum wins, and the window may exceed a very small screen rather than collapse
      below what the layout can render
- [+] 15 unit tests on the arithmetic, two of which read `tauri.conf.json` and
      `tokens.css` so the numbers restated in Rust cannot go stale in silence

## Step 2 — a zoom the user can reach

- [+] `Ctrl` `+` / `Ctrl` `-` / `Ctrl` `0` anywhere in the window
- [+] `Ctrl` + mouse wheel, which takes the gesture over from the webview's own zoom
      rather than adding to it
- [+] Both apply immediately **and persist**, through a command that writes only the
      two zoom fields — so it cannot smuggle the settings page's unsaved edits into the file
- [+] The shortcuts do not fire while the user is typing in an input, a textarea, a
      select or anything `contenteditable`
- [+] A zoom readout in the status bar with two buttons, marked `auto` while the factor
      is the monitor's rather than the user's
- [+] EN and RU strings, verified equal at 420 keys each with no orphans — the now-unused
      `settings.uiZoom.reset` was removed rather than left behind

## Step 3 — a first-run default that suits the monitor

- [+] `Config::ui_zoom_auto`, default `true`: derive the zoom from the monitor until the
      user adjusts it, and then leave it alone. Nothing is persisted on that path, which is
      what makes a different monitor adapt instead of inheriting the old one's factor
- [+] Adjusting zoom by any route sets it `false` — verified on the running app: after a
      keystroke the settings file read `ui_zoom 0.85, ui_zoom_auto false`
- [+] The derivation is a stated rule rather than a magic number: the largest zoom no
      greater than 1.0 at which the designed layout fits the work area, clamped to the
      existing range. It only ever zooms out — enlarging would override the display scale
      the user chose in their operating system
- [+] Tests for the derivation, including this monitor (0.87), a 1366×768 laptop (0.85),
      roomy monitors (no change at all) and nonsense input

## Step 4 — verification

- [+] `cargo xtask gate` green, 11/11 sections
- [+] **The application was actually run**, and it is what found the two defects below
      that no test had caught. Verified by measuring the live window and reading the
      settings file it wrote:
      - the window opens **1115×752 inside a 1280×752 work area** — confirmed on three
        separate launches, against 1129×789 before the frame fix
      - the derived zoom is applied and **not** persisted (no settings file until a
        keystroke wrote one)
      - `Ctrl -` took it 0.87 → 0.85 and persisted with `auto=false`
      - `Ctrl +` took it 0.85 → 0.90
- [-] Checked at more than one *physical* monitor. **Not done**: this machine has one
      display. The behaviour across sizes is covered by the unit tests, which is not the
      same thing as having seen it — a 4K panel and a 1366×768 laptop are asserted, not
      observed
- [-] `Ctrl 0`, `Ctrl`+wheel and the status-bar buttons verified by input. **Not
      verified**: synthetic keystrokes reached the webview only intermittently — the same
      `Ctrl -` both worked and failed within one run — and the dev-mode window kept being
      destroyed by the file watcher whenever anything in `src-tauri` was touched. The
      running binary was confirmed newer than the sources, so a stale build is ruled out.
      These are one keystroke each for a person at the machine

## What running it found, and nothing else would have

Three defects, none of which a test or a review had caught, and each found by measuring
rather than by reading:

1. **`set_size` takes an inner size; `outer_size` reports an outer one.** The first
   version read one and wrote the other, so the frame was added on every launch: asked
   for 752, the window came out 789 against a 752 work area — still not fitting, and now
   for a reason of this code's own making. The arithmetic is in one coordinate system
   now, with `fit_inner` as a pure function and a regression test carrying the measured
   frame.
2. **Stepping the zoom up was a silent no-op at six values.** `factor / STEP` is exact
   for most and not for all — `1.2 / 0.05` is `23.999999999999996` — so `Math.floor(…) + 1`
   landed back where it started at 0.70, 0.95, 1.15, 1.20, 1.40 and 1.45. **0.70 is the
   minimum**, exactly where somebody on a small screen would be pressing the key. It
   works in integer hundredths now.
3. **`Ctrl 0` could never return to the monitor's suggestion.** It asked `startup_zoom`,
   which honours `ui_zoom_auto` and therefore hands back the stored choice once one
   exists — the very factor the reset is trying to discard. It asks `monitor_zoom` now.

And one found while trying to run the app at all: **`pnpm desktop:dev` did not work.**
`pnpm --dir apps/desktop exec` fails because `apps/desktop` has no `package.json`. That
command was documented, corrected once before, and still did not run — a command nobody
executes is a command nobody knows is broken.

## Step 5 — completion

- [+] `docs/architecture/overview.md`: the desktop front-end paragraph now describes the
      window fit and where the scale comes from
- [+] `README.md`: the shortcuts and the monitor-matching behaviour
- [+] `CHANGELOG.md`: an entry under Unreleased, including the slider whose range did not
      match what the application accepts
- [+] `.ai/project/15-command-reference.md`: why `desktop:dev` was broken and what it is
      now; `AGENTS.md` unchanged, that module being `tier: extended`
- [+] Checklist filled with `+`/`-`
- [+] Final report
