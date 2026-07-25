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
