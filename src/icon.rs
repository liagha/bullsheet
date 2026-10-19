pub(crate) struct Icon {
    rgba: &'static [u8],
    width: u32,
    height: u32,
}

impl Icon {
    pub(crate) fn new() -> Self {
        Self {
            rgba: include_bytes!("../assets/bullsheet-64.rgba").as_slice(),
            width: 64,
            height: 64,
        }
    }

    pub(crate) fn data(&self) -> eframe::egui::IconData {
        eframe::egui::IconData {
            rgba: self.rgba.to_vec(),
            width: self.width,
            height: self.height,
        }
    }
}
