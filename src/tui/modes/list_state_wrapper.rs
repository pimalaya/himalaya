use tui_rs::widgets::ListState;

pub struct ListStateWrapper {
    pub state:   ListState,
    list_length: usize,
}

impl ListStateWrapper {
    pub fn new() -> Self {
        Self {
            state:       ListState::default(),
            list_length: 0,
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

                if selection > self.list_length - 1 {
                    selection = self.list_length - 1;
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
            if index >= self.list_length {
                self.state.select(Some(self.list_length - 1));
            } else {
                self.state.select(Some(index));
            }
        } else {
            self.state.select(Some(self.list_length - 1));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    pub fn update_length(&mut self, length: usize) {
        self.list_length = length;
    }

    pub fn get_selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }
}
