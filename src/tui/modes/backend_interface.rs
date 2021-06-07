use crossterm::event::Event;

use crate::tui::model::BackendActions;
use crate::tui::tabs::account_tab::AccountTab;

use tui_rs::backend::Backend;
use tui_rs::terminal::Frame;
use tui_rs::layout::Rect;

// TODO: Doc
pub trait BackendInterface {
    fn handle_event<'event>(
        &mut self,
        event: Event,
        account: &mut AccountTab<'event>,
    ) -> Option<BackendActions>;

    fn draw<'draw, B>(
        &mut self,
        frame: &mut Frame<B>,
        free_space: Rect,
        account: &mut AccountTab<'draw>,
    ) where
        B: Backend;
}
