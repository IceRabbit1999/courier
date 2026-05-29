pub mod dashboard;
pub mod friend;
pub mod hero;
pub mod item;
pub mod matches;
pub mod setting;
pub mod setup;

pub trait Screen {
    fn show(&mut self, ui: &mut egui::Ui);
}
