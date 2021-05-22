use std::io;
use tui_rs::backend::{Backend, CrosstermBackend};
use tui_rs::layout::{Constraint, Direction, Layout, Rect};
use tui_rs::terminal::Frame;
use tui_rs::widgets::{Block, Borders};
use tui_rs::Terminal;

// ===================
// Structs/Traits
// =================== */
/// This MailFrame is added to all frames of the TUI. It should be able to
/// customize their position and their size with the functions of this trait.
///
/// It has the lifetime called "frame" because we'll always use a block for
/// each frame.
struct MailFrame<'frame> {
    /// This variable holds the rectangle/frame where it should be placed.
    /// Normally it set by a rect which comes of a [Layout](struct.Layout.html).
    rect: Rect,

    /// Since each frame will inside a block, the user or you can customize how
    /// the block should look like. You just need to set this variable with the
    /// [set_block](struct.MailFrame.html#method.set_block) function.
    block: Block<'frame>,
}

impl<'frame> MailFrame<'frame> {
    /// Creates a new frame with the given rectangle/frame and the block for
    /// decoration. For more information please take a look into their [their
    /// definition](struct.MailFrame.html).
    fn new(rect: Rect, block: Block<'frame>) -> MailFrame<'frame> {
        MailFrame { rect, block }
    }

    fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Since each frame in the TUI is covered in a box, you can set a box how
    /// it should look like by using this function. If you want to know how to
    /// customize the block, take a look into its
    /// [documentation](struct.Block.html).
    ///
    /// # Example
    /// ```rust
    /// let mut mailframe = MailFrame::new();
    /// mailframe.set_block(Block::default());
    /// ```
    fn set_block(&mut self, block: Block<'frame>) {
        self.block = block;
    }
}

// ==============
// Functions
// ============== */
/// This struct includes the whole information of the TUI. Starting from the
/// sidebar to the listing of the mails. In the end, it should look like this
/// (default):
///
///     --- Tab 1 ---
///     |           |
///     -  Sidebar  -- Mail Listing -------------------
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     |           |                                 |
///     -----------------------------------------------
///
pub struct TUI<'tui>
{
    /// This variable holds the sidebar-frame of the whole TUI
    /// TODO: Add ascii art of the sidebar
    sidebar: MailFrame<'tui>,

    /// This variable holds the main
    /// TODO: Add ascii art to understand better what it is
    mail_listing: MailFrame<'tui>,
}

impl<'tui> TUI<'tui> 
{
    /// Creates a new TUI struct which already sets the appropriate constraints
    /// and places the frames correctly. It'll give the sidebar and the
    /// mail_listing a default value. The result can be seen
    /// [here](struct.TUI.html).
    /// TODO: Add tabs (accounts)
    pub fn new<B>(frame: &mut Frame<B>) -> TUI<'tui>
        where B: Backend
    {
        // Create the two frames for the sidebar and the mails:
        //  - One on the left (sidebar)
        //  - One on the right (mail listing)
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                    // For the sidebar (will be the second block => Index 0)
                    Constraint::Percentage(25),
                    // For the mails (will be the second block => Index 1)
                    Constraint::Percentage(75),
                ]
                .as_ref(),
            )
            // Use the given frame size to create the two blocks
            .split(frame.size());

        // Create the two desired main-frames
        let sidebar = MailFrame::new(
            layout[0],
            Block::default().title("Sidebar").borders(Borders::ALL),
        );

        let mail_listing = MailFrame::new(
            layout[1], 
            Block::default().title("Mails").borders(Borders::ALL));

        TUI {
            sidebar,
            mail_listing,
        }
    }

    /// Use this function to draw the whole TUI with the sidebar, mails and
    /// accounts.
    ///
    /// # Example:
    /// ```rust
    /// let stdout = io::stdout();
    /// let backend = CrosstermBackend::new(stdout);
    /// let mut terminal = Terminal::new(backend)?;
    ///
    /// // clear the terminal
    /// terminal.clear()?;
    ///
    /// // Draw the user interface
    /// terminal.draw(|frame| {
    ///     let tui = TUI::new(frame);
    ///     tui.draw(frame);
    /// })?;
    /// ```
    pub fn draw<B>(&self, frame: &'tui mut Frame<B>)
        where B: Backend
    {
        frame.render_widget(self.sidebar.block.clone(), self.sidebar.rect);
        frame.render_widget(self.mail_listing.block.clone(), self.mail_listing.rect);
    }
}

// Start the tui by preparing
pub fn run_tui() -> Result<(), io::Error> {
    println!("Starting tui");

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // clear the terminal
    terminal.clear()?;

    // Draw the user interface
    terminal.draw(|frame| {
        let tui = TUI::new(frame);
        tui.draw(frame);
    })?;

    Ok(())
}
