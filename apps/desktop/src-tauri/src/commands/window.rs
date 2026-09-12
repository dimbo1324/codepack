//! Window chrome the frontend cannot set for itself.

use tauri::{Manager, WebviewWindow};

use crate::error::{CommandError, CommandResult};

/// The range `Config::ui_zoom` is clamped to, restated from
/// [`codepack_core::config::UI_ZOOM_MIN`]/[`codepack_core::config::UI_ZOOM_MAX`] so the
/// window and the stored setting cannot disagree about what is valid.
fn clamp_zoom(factor: f64) -> f64 {
    if factor.is_finite() {
        factor.clamp(
            codepack_core::config::UI_ZOOM_MIN,
            codepack_core::config::UI_ZOOM_MAX,
        )
    } else {
        codepack_core::config::DEFAULT_UI_ZOOM
    }
}

/// Sets the webview's zoom factor.
///
/// Done natively rather than with a CSS transform because a transform scales layout but
/// not the native scrollbars, focus rings or text rendering — the result looks blurry at
/// non-integer factors, which is most of the useful range.
#[tauri::command]
pub fn set_ui_zoom(app: tauri::AppHandle, factor: f64) -> CommandResult<()> {
    let window: WebviewWindow = app
        .get_webview_window("main")
        .ok_or_else(|| CommandError::new("the main window is not open"))?;
    window
        .set_zoom(clamp_zoom(factor))
        .map_err(CommandError::new)
}

/// The zoom this launch should open at.
///
/// Two answers, and which one you get is `Config::ui_zoom_auto`:
///
/// * `false` — the user has chosen a zoom, so the stored factor is used verbatim. Nothing
///   here looks at the monitor, because a choice that gets recomputed on the next launch
///   is not a choice.
/// * `true` (the default) — derive it from the monitor's work area, every launch. That is
///   what makes moving to a different monitor adapt rather than keep a factor that suited
///   the old one, and it is why nothing is persisted on this path.
///
/// Computes rather than applies: `set_ui_zoom` remains the only thing that touches the
/// webview, so there is one place where the zoom actually changes.
#[tauri::command]
pub fn startup_zoom(app: tauri::AppHandle) -> CommandResult<f64> {
    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);
    if !config.ui_zoom_auto {
        return Ok(config.normalized_ui_zoom());
    }
    Ok(crate::window_fit::monitor_zoom(&app).unwrap_or(codepack_core::config::DEFAULT_UI_ZOOM))
}

/// What this monitor suggests, ignoring what the user has stored.
///
/// Distinct from [`startup_zoom`], which honours `ui_zoom_auto` and therefore returns the
/// stored choice once one exists. `Ctrl 0` needs the other answer: it asks to go back to
/// following the monitor, and asking `startup_zoom` would have handed it the very factor
/// it is trying to discard.
///
/// Falls back to the default when the monitor cannot be read or has room for the designed
/// layout — the same meaning `suggested_zoom` gives `None`.
#[tauri::command]
pub fn monitor_zoom(app: tauri::AppHandle) -> f64 {
    crate::window_fit::monitor_zoom(&app).unwrap_or(codepack_core::config::DEFAULT_UI_ZOOM)
}

/// Writes the zoom to the settings file, so it survives a restart.
///
/// Separate from [`set_ui_zoom`], which only touches the webview, because startup applies
/// a factor without recording it: the derived one is not a decision the user made, and
/// persisting it would quietly convert "follow my monitor" into "stay at 87% forever",
/// including after they plug in a bigger screen.
///
/// `auto` is `false` for every user-initiated change and `true` only for the reset, which
/// is the one route that asks for the monitor to be followed again.
///
/// Writes only these two fields, read-modify-write on the stored file — not the session's
/// configuration. The settings page holds unsaved edits until the user presses save, and
/// a zoom change must not smuggle those into the file behind their back.
#[tauri::command]
pub fn save_ui_zoom(factor: f64, auto: bool) -> CommandResult<()> {
    let paths = codepack_core::AppPaths::resolve()?;
    let mut config = codepack_core::config::load(&paths);
    config.ui_zoom = clamp_zoom(factor);
    config.ui_zoom_auto = auto;
    codepack_core::config::save(&paths, &config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::config::{DEFAULT_UI_ZOOM, UI_ZOOM_MAX, UI_ZOOM_MIN};

    #[test]
    fn a_factor_inside_the_range_is_used_as_is() {
        assert_eq!(clamp_zoom(1.2), 1.2);
        assert_eq!(clamp_zoom(UI_ZOOM_MIN), UI_ZOOM_MIN);
        assert_eq!(clamp_zoom(UI_ZOOM_MAX), UI_ZOOM_MAX);
    }

    #[test]
    fn a_factor_outside_the_range_is_clamped_to_the_same_bounds_the_config_uses() {
        // The window and the stored setting must agree, or reopening the app would
        // silently change the zoom the user last saw.
        assert_eq!(clamp_zoom(0.1), UI_ZOOM_MIN);
        assert_eq!(clamp_zoom(9.0), UI_ZOOM_MAX);
    }

    #[test]
    fn a_non_finite_factor_falls_back_rather_than_producing_an_unusable_window() {
        // `set_zoom(NaN)` would leave the window in an undefined state the user could
        // not recover from without editing the settings file by hand.
        assert_eq!(clamp_zoom(f64::NAN), DEFAULT_UI_ZOOM);
        assert_eq!(clamp_zoom(f64::INFINITY), DEFAULT_UI_ZOOM);
        assert_eq!(clamp_zoom(f64::NEG_INFINITY), DEFAULT_UI_ZOOM);
    }

    #[test]
    fn clamping_matches_the_configs_own_normalizer() {
        // Two implementations of the same rule would eventually disagree; this asserts
        // they do not.
        for factor in [0.1, 0.7, 1.0, 1.5, 3.0] {
            let config = codepack_core::config::Config {
                ui_zoom: factor,
                ..Default::default()
            };
            assert_eq!(clamp_zoom(factor), config.normalized_ui_zoom());
        }
    }
}
