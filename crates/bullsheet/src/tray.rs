use element::Failure;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const SHOW: &str = "show";
const AUTOSTART: &str = "autostart";
const QUIT: &str = "quit";

pub enum Action {
    Toggle,
    Autostart,
    Quit,
}

pub struct Tray {
    #[allow(dead_code)]
    icon: TrayIcon,
    autostart: CheckMenuItem,
    action: std::sync::Arc<std::sync::Mutex<Option<Action>>>,
}

impl Tray {
    pub fn new(context: &eframe::egui::Context, autostart: bool) -> Result<Self, Failure> {
        let show = MenuItem::with_id(SHOW, "Show", true, None);
        let startup = CheckMenuItem::with_id(AUTOSTART, "Start with system", true, autostart, None);
        let quit = MenuItem::with_id(QUIT, "Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &startup,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|error| Failure::new(error.to_string()))?;
        let source = crate::icon::Icon::new();
        let icon = Icon::from_rgba(source.rgba(), source.width(), source.height())
            .map_err(|error| Failure::new(error.to_string()))?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("bullsheet")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|error| Failure::new(error.to_string()))?;
        let action = std::sync::Arc::new(std::sync::Mutex::new(None));
        let click = std::sync::Arc::clone(&action);
        let click_context = context.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Ok(mut pending) = click.lock() {
                    *pending = Some(Action::Toggle);
                }
                click_context.request_repaint();
            }
        }));
        let selection = std::sync::Arc::clone(&action);
        let selection_context = context.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let pending = match event.id.0.as_str() {
                SHOW => Some(Action::Toggle),
                AUTOSTART => Some(Action::Autostart),
                QUIT => Some(Action::Quit),
                _ => None,
            };
            if let Some(pending) = pending {
                if let Ok(mut slot) = selection.lock() {
                    *slot = Some(pending);
                }
                selection_context.request_repaint();
            }
        }));
        Ok(Self {
            icon,
            autostart: startup,
            action,
        })
    }

    pub fn action(&self) -> Option<Action> {
        self.action.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn set_autostart(&self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }
}
