pub trait Station {
    fn show(&mut self, ui: &mut eframe::egui::Ui);

    fn focus(&mut self);
}
