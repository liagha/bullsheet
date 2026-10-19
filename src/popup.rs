#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Visibility {
    Visible,
    #[default]
    Hidden,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
        })
    }
}

impl Visibility {
    fn toggled(self) -> Self {
        match self {
            Self::Visible => Self::Hidden,
            Self::Hidden => Self::Visible,
        }
    }
}

pub(crate) struct Popup {
    visibility: Visibility,
}

impl Popup {
    pub(crate) fn new(visibility: Visibility) -> Self {
        Self { visibility }
    }

    pub(crate) fn visibility(&self) -> Visibility {
        self.visibility
    }

    pub(crate) fn show(&mut self, context: &eframe::egui::Context) {
        context.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
        context.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
        self.visibility = Visibility::Visible;
    }

    pub(crate) fn hide(&mut self, context: &eframe::egui::Context) {
        context.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(false));
        self.visibility = Visibility::Hidden;
    }

    pub(crate) fn toggle(&mut self, context: &eframe::egui::Context) {
        match self.visibility.toggled() {
            Visibility::Visible => self.show(context),
            Visibility::Hidden => self.hide(context),
        }
    }
}
