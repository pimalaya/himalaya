use tui_rs::widgets::Block;

pub trait MailFrame {
    fn new(title: String) -> Self;
    fn block(&self) -> Block;
}
