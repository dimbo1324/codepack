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

- [ ] Orientation: the previous task's checklist is complete and its gate was green
- [ ] Measure the real environment rather than guess (done above)
- [ ] Branch, and this checklist committed **before** the work

## Step 1 — the window fits the monitor it opens on

- [ ] At startup the shell reads the current monitor and shrinks the window to fit,
      treating the configured size as a maximum rather than a demand
- [ ] Re-centred afterwards, because a window resized from its top-left corner drifts
- [ ] A monitor smaller than `minWidth`/`minHeight` does not produce an unusable window:
      the minimum wins, and the window is allowed to exceed a very small screen rather
      than collapse below what the layout can render
- [ ] Unit tests on the fitting arithmetic, which is where an off-by-a-taskbar lives

## Step 2 — a zoom the user can reach

- [ ] `Ctrl` `+` / `Ctrl` `-` / `Ctrl` `0` anywhere in the window
- [ ] `Ctrl` + mouse wheel, which is what people reach for first
- [ ] Both apply immediately **and persist**, so the next launch opens the same way
- [ ] The shortcuts do not fire while the user is typing in a text field
- [ ] A zoom readout in the status bar with two buttons, so the feature is discoverable
      without knowing the shortcut
- [ ] EN and RU strings

## Step 3 — a first-run default that suits the monitor

- [ ] `Config::ui_zoom_auto`, default `true`: derive the zoom from the monitor until the
      user adjusts it, and then leave it alone
- [ ] Adjusting zoom by any route sets it `false` — an explicit choice must not be
      overwritten on the next launch
- [ ] The derivation is a stated rule rather than a magic number: the largest zoom at
      which the designed layout still fits the work area, clamped to the existing range
- [ ] Tests for the derivation, including the monitor this was found on (1280×800)

## Step 4 — verification

- [ ] `cargo xtask gate` green
- [ ] **The application actually run**, not merely compiled: the window fits, the
      shortcuts work, the readout updates, and the setting survives a restart
- [ ] Checked at more than one window size, since "adapts to different monitors" is the
      requirement and one size proves nothing

## Step 5 — completion

- [ ] `docs/architecture/overview.md` if the shape changed
- [ ] `README.md` if what a user can do changed
- [ ] `CHANGELOG.md` entry
- [ ] Checklist filled with `+`/`-`
- [ ] Final report
