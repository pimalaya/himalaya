pub mod cli;
pub mod model;

pub mod tui {

    pub mod block_data;
    pub mod tui;

    pub mod modes {
        pub mod normal;
        pub mod viewer;
        pub mod writer;
    }
}
