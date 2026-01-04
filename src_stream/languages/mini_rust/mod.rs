pub mod values;
mod numeric;
pub mod structure;
pub mod expressions;
pub mod statements;
pub mod registry;
pub mod prelude;
pub mod src_mini_rust;

pub use src_mini_rust::register_all;
