use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use masonry::core::NewWidget;
use masonry::dpi::LogicalSize;
use masonry::peniko::Color;
use masonry::theme::default_property_set;
use masonry_winit::app::{
    AppDriver, DriverCtx, EventLoop, EventLoopProxy, MasonryState, MasonryUserEvent, NewWindow,
    WindowId,
};
use masonry_winit::winit::window::{Icon as WindowIcon, Window, WindowLevel};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::fonts::Typefaces;
use crate::instance::Instance;
use crate::settings::Settings;
use crate::sheet::{Sheet, SheetAction};

const ICON: &[u8] = include_bytes!("../assets/bullsheet-64.rgba");

#[derive(Debug, Clone, Copy)]
pub enum ShellAction {
    Toggle,
    Autostart,
    Wake,
    Blink,
    Quit,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayCommand {
    Show,
    Autostart,
    Quit,
}

impl TrayCommand {
    fn id(self) -> element::Text {
        match self {
            Self::Show => element::Text::new("show"),
            Self::Autostart => element::Text::new("autostart"),
            Self::Quit => element::Text::new("quit"),
        }
    }

    fn label(self) -> element::Text {
        match self {
            Self::Show => element::Text::new("Show / Hide"),
            Self::Autostart => element::Text::new("Start with system"),
            Self::Quit => element::Text::new("Quit"),
        }
    }

    fn from_id(id: &element::Text) -> Option<Self> {
        match id.as_ref() {
            "show" => Some(Self::Show),
            "autostart" => Some(Self::Autostart),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    fn action(self) -> ShellAction {
        match self {
            Self::Show => ShellAction::Toggle,
            Self::Autostart => ShellAction::Autostart,
            Self::Quit => ShellAction::Quit,
        }
    }
}

#[derive(Clone)]
pub struct Dispatcher {
    proxy: EventLoopProxy,
    window: WindowId,
}

impl Dispatcher {
    fn new(proxy: EventLoopProxy, window: WindowId) -> Self {
        Self { proxy, window }
    }

    fn window(&self) -> WindowId {
        self.window
    }

    fn send(&self, action: ShellAction) {
        let _ = self.proxy.send_event(MasonryUserEvent::Action(
            self.window,
            Box::new(action),
            masonry::core::WidgetId::next(),
        ));
    }
}

pub struct Tray {
    _icon: TrayIcon,
    autostart: CheckMenuItem,
}

impl Tray {
    fn new(autostart: bool) -> Option<Self> {
        let show = MenuItem::with_id(
            TrayCommand::Show.id().as_ref(),
            TrayCommand::Show.label().as_ref(),
            true,
            None,
        );
        let startup = CheckMenuItem::with_id(
            TrayCommand::Autostart.id().as_ref(),
            TrayCommand::Autostart.label().as_ref(),
            true,
            autostart,
            None,
        );
        let quit = MenuItem::with_id(
            TrayCommand::Quit.id().as_ref(),
            TrayCommand::Quit.label().as_ref(),
            true,
            None,
        );
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &startup,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .ok()?;
        let icon = tray_icon::Icon::from_rgba(ICON.to_vec(), 64, 64).ok()?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("bullsheet")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .ok()?;
        Some(Self {
            _icon: icon,
            autostart: startup,
        })
    }

    fn autostart_item(&self) -> CheckMenuItem {
        self.autostart.clone()
    }
}

pub struct Driver {
    dispatcher: Dispatcher,
    settings: Settings,
    tray: Option<Tray>,
}

impl Driver {
    fn toggle(&self, ctx: &mut DriverCtx<'_, '_>) {
        let handle = ctx.window(self.dispatcher.window()).handle();
        if handle.is_visible() == Some(true) {
            handle.set_visible(false);
        } else {
            handle.set_visible(true);
            handle.focus_window();
            handle.request_redraw();
        }
    }

    fn blink(&self, ctx: &mut DriverCtx<'_, '_>) {
        let handle = ctx.window(self.dispatcher.window()).handle();
        if handle.is_visible() != Some(true) || !handle.has_focus() {
            return;
        }
        let id = ctx.render_root(self.dispatcher.window()).get_layer_root(0).id();
        ctx.render_root(self.dispatcher.window())
            .edit_widget(id, |mut widget| {
                let mut sheet = widget.downcast::<Sheet>();
                if sheet.widget.tick_blink() {
                    sheet.ctx.request_paint_only();
                }
            });
    }
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: masonry::core::WidgetId,
        action: masonry::core::ErasedAction,
    ) {
        if let Some(action) = action.downcast_ref::<SheetAction>() {
            match action {
                SheetAction::DragWindow => {
                    let _ = ctx.window(window_id).handle().drag_window();
                }
                SheetAction::Hide => {
                    ctx.window(window_id).handle().set_visible(false);
                }
                SheetAction::Quit => {
                    ctx.exit();
                }
            }
            return;
        }

        let Some(action) = action.downcast_ref::<ShellAction>() else {
            return;
        };
        match action {
            ShellAction::Toggle => self.toggle(ctx),
            ShellAction::Autostart => {
                let enabled = !self.settings.autostart_enabled();
                if let Err(error) = self.settings.set_autostart(enabled) {
                    eprintln!("could not change autostart: {error}");
                }
            }
            ShellAction::Wake => {
                ctx.window(window_id).handle().request_redraw();
            }
            ShellAction::Blink => self.blink(ctx),
            ShellAction::Quit => ctx.exit(),
        }
    }

    fn on_start(&mut self, state: &mut MasonryState<'_>) {
        let typefaces = Typefaces::new();
        for root in state.roots() {
            typefaces.register(root);
            let id = root.get_layer_root(0).id();
            root.focus_on(Some(id));
        }

        let enabled = self.settings.autostart_enabled();
        self.tray = Tray::new(enabled);
        if let Some(tray) = &self.tray {
            let item = tray.autostart_item();
            self.settings.on_autostart(move |enabled| {
                item.set_checked(enabled);
            });
        }

        let click_dispatcher = self.dispatcher.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                click_dispatcher.send(ShellAction::Toggle);
            }
        }));

        let menu_dispatcher = self.dispatcher.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let Some(command) = TrayCommand::from_id(&element::Text::new(event.id.0.as_str())) else {
                return;
            };
            menu_dispatcher.send(command.action());
        }));

        let hotkey_dispatcher = self.dispatcher.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                hotkey_dispatcher.send(ShellAction::Toggle);
            }
        }));
    }
}

pub struct Application;

impl Application {
    pub fn run() {
        let _instance = match Instance::new() {
            Ok(Some(instance)) => Some(instance),
            Ok(None) => return,
            Err(error) => {
                eprintln!("{error}");
                None
            }
        };

        let mut hidden = std::env::args().any(|argument| argument == "--hidden");

        let window = WindowId::next();
        let event_loop = EventLoop::with_user_event().build().unwrap();
        let dispatcher = Dispatcher::new(event_loop.create_proxy(), window);
        let wake = {
            let dispatcher = dispatcher.clone();
            move || dispatcher.send(ShellAction::Wake)
        };
        let settings = Settings::new(wake);
        hidden &= settings.registered();

        let sheet = Sheet::new(settings.clone(), Typefaces::new());

        let blink_dispatcher = dispatcher.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            blink_dispatcher.send(ShellAction::Blink);
        });

        let icon = WindowIcon::from_rgba(ICON.to_vec(), 64, 64).ok();
        let mut attributes = Window::default_attributes()
            .with_title("bullsheet")
            .with_inner_size(LogicalSize::new(640.0, 440.0))
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_visible(!hidden)
            .with_active(!hidden)
            .with_window_icon(icon);
        #[cfg(target_os = "windows")]
        {
            use masonry_winit::winit::platform::windows::WindowAttributesExtWindows;
            attributes = attributes.with_skip_taskbar(true);
        }

        let _ = masonry_winit::app::run_with(
            event_loop,
            vec![
                NewWindow::new_with_id(window, attributes, NewWidget::new(sheet).erased())
                    .with_base_color(Color::from_rgb8(32, 32, 32)),
            ],
            Driver {
                dispatcher,
                settings,
                tray: None,
            },
            default_property_set(),
        );
    }
}
