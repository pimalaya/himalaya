pub mod himalaya_tui {

    use std::io;
    use tui::backend::CrosstermBackend;
    use tui::Terminal;

    pub fn run() -> Result<(), io::Error> {

        // Prepare the terminal by over taking stdout
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        Ok(())
    }
}
