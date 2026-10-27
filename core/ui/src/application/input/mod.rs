//! Input routing and the chrome controller that consumes it.

mod input_router;
mod modifier_policy;
mod ui_controller;

#[cfg(test)]
mod tests;

pub use input_router::InputRouter;
pub use ui_controller::UiController;
