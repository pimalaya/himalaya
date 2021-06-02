use crossterm::event::Event;

use crate::tui::model::BackendActions;

use tui_rs::backend::Backend;
use tui_rs::terminal::Frame;

// TODO: Doc
pub trait BackendInterface {
    fn handle_event(&mut self, event: Event) -> Option<BackendActions>;
    fn draw<B: Backend>(&mut self, frame: &mut Frame<B>);
}
