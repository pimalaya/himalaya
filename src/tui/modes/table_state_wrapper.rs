use tui_rs::widgets::TableState;

pub struct TableStateWrapper {
    pub state:    TableState,
    table_length: usize,
}

impl TableStateWrapper {
    pub fn new() -> Self {
        Self {
            state:        TableState::default(),
            table_length: 0,
        }
    }

    pub fn move_cursor(&mut self, offset: i32) {
        let new_selection = match self.state.selected() {
            Some(old_selection) => {
                let mut selection = if offset < 0 {
                    old_selection.saturating_sub(offset.abs() as usize)
                } else {
                    old_selection.saturating_add(offset as usize)
                };

                if selection > self.table_length - 1 {
                    selection = self.table_length - 1;
                }

                selection
            },
            // If something goes wrong: Move the cursor to the beginning of the
            // selections.
            None => 0,
        };

        self.state.select(Some(new_selection));
    }

    pub fn set_cursor(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            if index >= self.table_length {
                self.state.select(Some(self.table_length - 1));
            } else {
                self.state.select(Some(index));
            }
        } else {
            self.state.select(Some(self.table_length - 1));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    pub fn update_length(&mut self, length: usize) {
        self.table_length = length;
    }
}
