use iui::controls::{Button, VerticalBox};
use iui::prelude::*;

fn main() {
    let ui = UI::init().expect("Couldn't initialize UI library");
    let mut win = Window::new(&ui, "Himalaya", 200, 200, WindowType::NoMenubar);

    let mut vbox = VerticalBox::new(&ui);
    vbox.set_padded(&ui, true);

    let mut quit_button = Button::new(&ui, "Quit");
    quit_button.on_clicked(&ui, {
        let ui = ui.clone();
        move |_| {
            ui.quit();
        }
    });

    vbox.append(&ui, quit_button, LayoutStrategy::Compact);

    win.set_child(&ui, vbox);
    win.show(&ui);
    ui.main();
}
