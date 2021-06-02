pub mod cli;
pub mod model;

pub mod modes {
    pub mod normal {
        pub mod mail_list;
        pub mod main;
        pub mod sidebar;
    }

    pub mod writing_mail {
        pub mod attachments;
        pub mod mail_credits;
        pub mod main;
    }

    pub mod backend_interface;
    pub mod block_data;
    pub mod keybinding_manager;
    // pub mod tui_widget;
}
