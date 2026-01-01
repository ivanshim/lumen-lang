pub mod assignment;
pub mod print;
pub mod if_else;
pub mod while_loop;
pub mod break_stmt;
pub mod continue_stmt;

pub fn register_all(registry: &mut crate::kernel::registry::Registry) {
    assignment::register(registry);
    print::register(registry);
    if_else::register(registry);
    while_loop::register(registry);
    break_stmt::register(registry);
    continue_stmt::register(registry);
}
