#![cfg_attr(target_os = "macos", allow(clippy::unused_unit))]

use tauri::{
    App, AppHandle, LogicalSize, Manager, Runtime, WebviewWindow, Window, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::monitor_refresh;

#[cfg(target_os = "macos")]
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt, tauri_panel,
};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(UsagePanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            becomes_key_only_if_needed: true,
            is_floating_panel: true
        }
    })

    panel_event!(UsagePanelEventHandler {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const USAGE_TRAY_ID: &str = "usage";

const TOGGLE_MENU_ID: &str = "toggle_usage";
const QUIT_MENU_ID: &str = "quit";

#[derive(Debug, Clone, Copy, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WindowMode {
    Compact,
    Detailed,
}

impl WindowMode {
    fn logical_size(self) -> LogicalSize<f64> {
        match self {
            Self::Compact => LogicalSize::new(340.0, 82.0),
            Self::Detailed => LogicalSize::new(420.0, 510.0),
        }
    }
}

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        setup_macos_panel(app)?;
    }

    let toggle_item =
        MenuItem::with_id(app, TOGGLE_MENU_ID, "显示 / 隐藏用量", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, QUIT_MENU_ID, "退出 Codex Reserve", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;

    let tray_builder = TrayIconBuilder::with_id(USAGE_TRAY_ID)
        .title("--")
        .tooltip("Codex Reserve · 等待真实用量数据")
        .icon(tray_ring_icon(None))
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TOGGLE_MENU_ID => toggle_usage_window(app),
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_usage_window(tray.app_handle());
            }
        });

    tray_builder.build(app)?;
    monitor_refresh::start_usage_refresh_ticker(app.app_handle().clone())?;

    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        apply_window_mode(&window, WindowMode::Compact)?;
        show_usage_window(app.app_handle());
    }

    #[cfg(target_os = "linux")]
    show_usage_window(app.app_handle());

    Ok(())
}

pub fn tray_ring_icon(remaining_percent: Option<u8>) -> Image<'static> {
    const SIZE: u32 = 32;
    const CENTER: f32 = 15.5;
    const RADIUS: f32 = 11.5;
    const HALF_STROKE: f32 = 2.1;

    let mut rgba = vec![0; (SIZE * SIZE * 4) as usize];
    let progress = remaining_percent.map(|percent| percent as f32 / 100.0);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - CENTER;
            let dy = y as f32 - CENTER;
            let distance = (dx * dx + dy * dy).sqrt();
            let edge_coverage = (HALF_STROKE + 0.75 - (distance - RADIUS).abs()).clamp(0.0, 1.0);
            if edge_coverage <= 0.0 {
                continue;
            }

            let mut angle = dx.atan2(-dy);
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            let position = angle / std::f32::consts::TAU;
            let alpha = match progress {
                Some(value) if position <= value => 255.0,
                Some(_) => 46.0,
                None => 110.0,
            };
            let pixel = ((y * SIZE + x) * 4) as usize;
            rgba[pixel + 3] = (alpha * edge_coverage).round() as u8;
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        hide_usage_window(window.app_handle());
    }
}

pub fn show_usage_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    #[cfg(target_os = "macos")]
    {
        if let Err(error) = window.move_window_constrained(Position::TrayBottomCenter) {
            eprintln!(
                "unable to position usage panel below the tray icon, falling back to top-right: {error}"
            );
            let _ = window.move_window(Position::TopRight);
        }

        if let Ok(panel) = app.get_webview_panel(MAIN_WINDOW_LABEL) {
            panel.show_and_make_key();
        }
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let _ = window.move_window(Position::TopRight);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_usage_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(panel) = app.get_webview_panel(MAIN_WINDOW_LABEL) {
            panel.hide();
        }
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

pub fn apply_window_mode<R: Runtime>(
    window: &WebviewWindow<R>,
    mode: WindowMode,
) -> tauri::Result<()> {
    window.set_size(mode.logical_size())?;

    #[cfg(target_os = "windows")]
    window.move_window(Position::TopRight)?;

    Ok(())
}

fn toggle_usage_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let Ok(panel) = app.get_webview_panel(MAIN_WINDOW_LABEL) else {
            return;
        };

        if panel.is_visible() {
            panel.hide();
        } else {
            show_usage_window(app);
        }
    }

    #[cfg(not(target_os = "macos"))]
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    #[cfg(not(target_os = "macos"))]
    if window.is_visible().unwrap_or(false) {
        hide_usage_window(app);
    } else {
        show_usage_window(app);
    }
}

#[cfg(target_os = "macos")]
fn setup_macos_panel(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or("main webview window is missing")?;
    let panel = window.to_panel::<UsagePanel>()?;

    panel.set_level(PanelLevel::Floating.value());
    panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .ignores_cycle()
            .transient()
            .into(),
    );
    panel.set_hides_on_deactivate(false);
    panel.set_works_when_modal(true);
    panel.set_released_when_closed(false);

    let handler = UsagePanelEventHandler::new();
    let app_handle = app.app_handle().clone();
    handler.window_did_resign_key(move |_notification| {
        hide_usage_window(&app_handle);
    });
    panel.set_event_handler(Some(handler.as_ref()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_mode_stays_glanceable() {
        assert_eq!(
            WindowMode::Compact.logical_size(),
            LogicalSize::new(340.0, 82.0)
        );
    }

    #[test]
    fn detailed_mode_has_room_for_planning_information() {
        assert_eq!(
            WindowMode::Detailed.logical_size(),
            LogicalSize::new(420.0, 510.0)
        );
    }

    #[test]
    fn tray_ring_uses_the_weekly_percentage_as_an_arc() {
        let empty = tray_ring_icon(Some(0));
        let nearly_full = tray_ring_icon(Some(90));
        let empty_opaque_pixels = empty
            .rgba()
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|&&a| a > 200)
            .count();
        let full_opaque_pixels = nearly_full
            .rgba()
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|&&a| a > 200)
            .count();

        assert!(full_opaque_pixels > empty_opaque_pixels);
        assert_eq!(empty.width(), 32);
        assert_eq!(empty.height(), 32);
    }
}
