use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Refresh,
    Settings,
    Exit,
}

pub struct Tray {
    _tray_icon: tray_icon::TrayIcon,
}

const MENU_ID_REFRESH: &str = "refresh";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_EXIT: &str = "exit";

impl Tray {
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();
        menu.append(&MenuItem::with_id(MENU_ID_REFRESH, "刷新", true, None))
            .map_err(|e| e.to_string())?;
        menu.append(&MenuItem::with_id(MENU_ID_SETTINGS, "设置", true, None))
            .map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| e.to_string())?;
        menu.append(&MenuItem::with_id(MENU_ID_EXIT, "退出", true, None))
            .map_err(|e| e.to_string())?;

        let icon = default_tray_icon().map_err(|e| format!("tray icon error: {e}"))?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("RightCode Floating Ball")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            _tray_icon: tray_icon,
        })
    }
}

pub fn drain_actions() -> Vec<TrayAction> {
    let mut actions = Vec::new();

    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let action = match event.id.as_ref() {
            MENU_ID_REFRESH => Some(TrayAction::Refresh),
            MENU_ID_SETTINGS => Some(TrayAction::Settings),
            MENU_ID_EXIT => Some(TrayAction::Exit),
            _ => None,
        };

        if let Some(action) = action {
            actions.push(action);
        }
    }

    actions
}

fn default_tray_icon() -> Result<Icon, tray_icon::BadIcon> {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center = (size as f32 - 1.0) / 2.0;
    let radius = (size as f32 / 2.0) - 1.0;
    let border = 1.4;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let d = (dx * dx + dy * dy).sqrt();

            if d > radius {
                continue;
            }

            let t = (1.0 - (d / radius)).clamp(0.0, 1.0);

            let fill_r = (0.0 * (1.0 - t) + 30.0 * t) as u8;
            let fill_g = (200.0 * (1.0 - t) + 255.0 * t) as u8;
            let fill_b = (180.0 * (1.0 - t) + 220.0 * t) as u8;

            let (r, g, b, a) = if d >= radius - border {
                (0, 255, 170, 255)
            } else {
                (fill_r, fill_g, fill_b, 255)
            };

            let idx = ((y * size + x) * 4) as usize;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = a;
        }
    }

    Icon::from_rgba(rgba, size, size)
}
