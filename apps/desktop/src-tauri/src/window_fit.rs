//! Opening the window at a size the monitor can actually show, and picking a first-run
//! zoom that suits it.
//!
//! ## Why this exists
//!
//! `tauri.conf.json` asks for 1100×760 logical pixels, and until 2026-09-12 that is what
//! the window got on every machine. On the monitor this was found on — 1920×1200 physical
//! at 150% Windows scaling, so 1280×800 logical with a 1280×752 work area — the window
//! was **taller than the space it opened into**. A configured size is an intent, not a
//! measurement of somebody else's screen.
//!
//! ## The two rules, and why they are separate
//!
//! [`fit`] is about correctness: a window must not exceed its monitor's work area, and
//! must not shrink below the minimum the layout can render. It only ever makes the window
//! smaller than configured, never larger — a big monitor is not a reason to inflate a
//! window the author sized deliberately.
//!
//! [`suggested_zoom`] is about comfort, and is deliberately timid. It only ever zooms
//! *out*, only on a logical desktop too small for the designed layout, and only until the
//! user adjusts the zoom themselves — after that `Config::ui_zoom_auto` is `false` and
//! this is never consulted again. Zooming *in* automatically would override a display
//! scale the user chose in their operating system, which is not this application's
//! decision to make.

use tauri::{LogicalSize, Manager, PhysicalPosition, PhysicalSize};

use codepack_core::config::{DEFAULT_UI_ZOOM, UI_ZOOM_MAX, UI_ZOOM_MIN};

/// Width the designed layout wants: the sidebar plus the content column's maximum.
///
/// Both numbers come from `styles/tokens.css` (`--layout-sidebar: 232px`,
/// `--layout-content-max: 1080px`) and are restated rather than read, because CSS is not
/// available here. A test asserts the sum, so a token change that makes this stale fails
/// loudly instead of silently choosing a worse zoom.
const DESIGN_WIDTH: f64 = 232.0 + 1080.0;

/// Height the designed layout wants: the header, the status bar, and enough content
/// between them to show a card and its body without scrolling immediately.
const DESIGN_HEIGHT: f64 = 52.0 + 28.0 + 780.0;

/// The size the window should open at.
///
/// Per axis: no larger than the work area, no larger than configured, and never below the
/// minimum. The minimum wins last on purpose — on a monitor smaller than the layout can
/// render, a window that overflows the screen is recoverable (the user can move it) while
/// a window squeezed below `minWidth` is not (the layout collapses into itself).
pub(crate) fn fit(
    configured: LogicalSize<f64>,
    work_area: LogicalSize<f64>,
    minimum: LogicalSize<f64>,
) -> LogicalSize<f64> {
    LogicalSize::new(
        configured.width.min(work_area.width).max(minimum.width),
        configured.height.min(work_area.height).max(minimum.height),
    )
}

/// The zoom a first run should use on a monitor this size, or `None` to leave the default
/// alone.
///
/// The rule, stated so it is not a magic number: **the largest zoom no greater than 1.0
/// at which the designed layout fits the work area**, clamped to the range
/// `Config::ui_zoom` already permits. On a monitor with room for the design this returns
/// `None` and nothing happens; on a smaller logical desktop it returns the factor that
/// makes the design fit.
///
/// Worked example, the monitor that prompted this: a 1280×752 work area gives
/// `min(1.0, 1280/1312, 752/860)` = `0.874`, so the interface opens at 87% and the layout
/// has 1471 CSS pixels of width to work with instead of 1280.
pub(crate) fn suggested_zoom(work_area: LogicalSize<f64>) -> Option<f64> {
    if !work_area.width.is_finite() || !work_area.height.is_finite() {
        return None;
    }
    if work_area.width <= 0.0 || work_area.height <= 0.0 {
        return None;
    }

    let by_width = work_area.width / DESIGN_WIDTH;
    let by_height = work_area.height / DESIGN_HEIGHT;
    let fitted = by_width.min(by_height).min(DEFAULT_UI_ZOOM);

    // Nothing to do when the design already fits: returning `Some(1.0)` would be the same
    // as the default, but it would also mark the configuration as "derived", which is a
    // claim about a decision that was not made.
    if fitted >= DEFAULT_UI_ZOOM {
        return None;
    }
    Some(round_to_hundredth(fitted.clamp(UI_ZOOM_MIN, UI_ZOOM_MAX)))
}

/// Two decimal places, so the stored setting and the readout in the status bar agree and
/// the settings file stays readable.
fn round_to_hundredth(factor: f64) -> f64 {
    (factor * 100.0).round() / 100.0
}

/// Applies [`fit`] to the real window, then re-centres it.
///
/// Best-effort throughout: every failure here leaves the window at its configured size,
/// which is what it did before this function existed. A window that is the wrong size is
/// a nuisance; refusing to start over one would be worse.
pub(crate) fn fit_main_window(app: &tauri::AppHandle) -> Option<()> {
    let window = app.get_webview_window("main")?;
    let monitor = window.current_monitor().ok().flatten()?;

    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let area = monitor.work_area();
    let work_area = LogicalSize::new(
        f64::from(area.size.width) / scale,
        f64::from(area.size.height) / scale,
    );

    let configured: LogicalSize<f64> = window.outer_size().ok()?.to_logical(scale);
    let minimum = LogicalSize::new(MIN_WIDTH, MIN_HEIGHT);
    let target = fit(configured, work_area, minimum);

    // Only touch the window when the numbers actually differ: `set_size` on a window that
    // already has that size still generates a resize event, and the frontend listens for
    // those.
    if (target.width - configured.width).abs() >= 1.0
        || (target.height - configured.height).abs() >= 1.0
    {
        window.set_size(target).ok()?;
        centre_in(&window, area.position, area.size, target, scale);
    }
    Some(())
}

/// The zoom [`suggested_zoom`] derives from the monitor the window is currently on.
///
/// Read-only: it measures and returns. `None` when the monitor cannot be read or has room
/// for the design, which the caller renders as "leave the zoom at 1.0".
pub(crate) fn monitor_zoom(app: &tauri::AppHandle) -> Option<f64> {
    let window = app.get_webview_window("main")?;
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let area = monitor.work_area();
    suggested_zoom(LogicalSize::new(
        f64::from(area.size.width) / scale,
        f64::from(area.size.height) / scale,
    ))
}

/// `minWidth`/`minHeight` from `tauri.conf.json`, restated for the same reason
/// [`DESIGN_WIDTH`] is: this code cannot read that file. A test asserts they still match.
const MIN_WIDTH: f64 = 880.0;
const MIN_HEIGHT: f64 = 600.0;

/// Puts the window in the middle of the work area rather than the middle of the monitor,
/// so it does not sit under the taskbar on a screen that has one along an edge.
fn centre_in(
    window: &tauri::WebviewWindow,
    area_position: PhysicalPosition<i32>,
    area_size: PhysicalSize<u32>,
    size: LogicalSize<f64>,
    scale: f64,
) {
    let physical = PhysicalSize::new(
        (size.width * scale).round() as i32,
        (size.height * scale).round() as i32,
    );
    let x =
        area_position.x + (i32::try_from(area_size.width).unwrap_or(i32::MAX) - physical.width) / 2;
    let y = area_position.y
        + (i32::try_from(area_size.height).unwrap_or(i32::MAX) - physical.height) / 2;
    let _ = window.set_position(PhysicalPosition::new(
        x.max(area_position.x),
        y.max(area_position.y),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: f64, height: f64) -> LogicalSize<f64> {
        LogicalSize::new(width, height)
    }

    /// The configured size and the minimum are duplicated from `tauri.conf.json`, so
    /// something has to fail when that file changes. This is that something.
    #[test]
    fn the_restated_window_numbers_still_match_the_manifest() {
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"),
        )
        .expect("the shell's own manifest");
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).expect("a readable manifest");
        let main = &parsed["app"]["windows"][0];

        assert_eq!(main["minWidth"].as_f64(), Some(MIN_WIDTH));
        assert_eq!(main["minHeight"].as_f64(), Some(MIN_HEIGHT));
    }

    /// And the design figures against the stylesheet that owns them.
    #[test]
    fn the_restated_layout_numbers_still_match_the_tokens() {
        let tokens = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/styles/tokens.css"),
        )
        .expect("the token file");

        for (token, expected) in [
            ("--layout-sidebar", 232.0),
            ("--layout-content-max", 1080.0),
        ] {
            let line = tokens
                .lines()
                .find(|line| line.trim_start().starts_with(token))
                .unwrap_or_else(|| panic!("{token} is gone from tokens.css"));
            let value: f64 = line
                .split(':')
                .nth(1)
                .and_then(|rest| {
                    rest.trim()
                        .trim_end_matches(";")
                        .trim_end_matches("px")
                        .parse()
                        .ok()
                })
                .unwrap_or_else(|| panic!("cannot read {token}: {line}"));
            assert_eq!(
                value, expected,
                "{token} changed; DESIGN_WIDTH is now stale"
            );
        }
        assert_eq!(DESIGN_WIDTH, 232.0 + 1080.0);
    }

    #[test]
    fn a_window_that_fits_is_left_alone() {
        // The common case: a monitor with room to spare must not be second-guessed.
        let fitted = fit(
            size(1100.0, 760.0),
            size(1920.0, 1040.0),
            size(880.0, 600.0),
        );
        assert_eq!((fitted.width, fitted.height), (1100.0, 760.0));
    }

    #[test]
    fn a_window_taller_than_the_work_area_is_shortened() {
        // The bug this module was written for: 760 configured against 752 available.
        let fitted = fit(size(1100.0, 760.0), size(1280.0, 752.0), size(880.0, 600.0));
        assert_eq!((fitted.width, fitted.height), (1100.0, 752.0));
    }

    #[test]
    fn a_big_monitor_does_not_inflate_the_window() {
        // Configured is a ceiling as well as a wish; 4K is not a reason to open at 4K.
        let fitted = fit(
            size(1100.0, 760.0),
            size(3840.0, 2100.0),
            size(880.0, 600.0),
        );
        assert_eq!((fitted.width, fitted.height), (1100.0, 760.0));
    }

    #[test]
    fn the_minimum_wins_over_a_monitor_too_small_for_it() {
        // A window overflowing a tiny screen can be moved; one squeezed under `minWidth`
        // renders a collapsed layout the user cannot fix.
        let fitted = fit(size(1100.0, 760.0), size(800.0, 480.0), size(880.0, 600.0));
        assert_eq!((fitted.width, fitted.height), (880.0, 600.0));
    }

    #[test]
    fn a_roomy_monitor_suggests_no_zoom_change_at_all() {
        // `None`, not `Some(1.0)`: the difference is whether the configuration gets
        // marked as derived, which is a claim about a decision nobody made.
        assert_eq!(suggested_zoom(size(1920.0, 1040.0)), None);
        assert_eq!(suggested_zoom(size(3840.0, 2100.0)), None);
    }

    #[test]
    fn the_monitor_this_was_found_on_suggests_zooming_out() {
        // 1280×752 work area: min(1.0, 1280/1312, 752/860) = 0.874.
        assert_eq!(suggested_zoom(size(1280.0, 752.0)), Some(0.87));
    }

    #[test]
    fn a_small_laptop_screen_suggests_zooming_out_further() {
        // 1366×768 at 100%, a very common panel: bound by height, not width.
        assert_eq!(suggested_zoom(size(1366.0, 728.0)), Some(0.85));
    }

    #[test]
    fn the_suggestion_never_goes_below_the_range_the_config_permits() {
        // A phone-sized work area would compute far below `UI_ZOOM_MIN`; the window and
        // the stored setting have to agree on what is valid.
        let suggested = suggested_zoom(size(400.0, 300.0)).expect("a suggestion");
        assert_eq!(suggested, UI_ZOOM_MIN);
    }

    #[test]
    fn the_suggestion_never_zooms_in() {
        // Enlarging automatically would override the display scale the user chose in
        // their operating system.
        for (width, height) in [(1920.0, 1040.0), (2560.0, 1400.0), (3840.0, 2100.0)] {
            assert!(
                suggested_zoom(size(width, height)).is_none_or(|zoom| zoom <= DEFAULT_UI_ZOOM),
                "{width}x{height} suggested zooming in"
            );
        }
    }

    #[test]
    fn nonsense_monitor_dimensions_suggest_nothing() {
        assert_eq!(suggested_zoom(size(0.0, 0.0)), None);
        assert_eq!(suggested_zoom(size(-1.0, 500.0)), None);
        assert_eq!(suggested_zoom(size(f64::NAN, 500.0)), None);
        assert_eq!(suggested_zoom(size(f64::INFINITY, 500.0)), None);
    }
}
