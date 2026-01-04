use crate::languages::mini_rust::prelude::*;
pub mod literals;
pub mod arithmetic;
pub mod comparison;
pub mod logic;
pub mod variable;
pub mod identifier;
pub mod grouping;

pub fn register_all(registry: &mut crate::languages::mini_rust::registry::Registry) {
    literals::register(registry);
    arithmetic::register(registry);
    comparison::register(registry);
    logic::register(registry);
    variable::register(registry);
    identifier::register(registry);
    grouping::register(registry);
}
