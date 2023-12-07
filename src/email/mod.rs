pub mod envelope;
pub mod message;

#[doc(inline)]
pub use self::{
    envelope::flag,
    message::{attachment, template},
};
