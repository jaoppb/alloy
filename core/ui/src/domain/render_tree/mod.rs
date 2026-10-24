//! The browser UI render tree: nodes, hit targets and the geometry they are
//! tested with. Behavior-preserving move out of `alloy` (#109).

mod builder;
mod hit_geometry;
mod hit_target;
mod node;
mod node_children;
mod node_kind;
mod surfaces;
mod tree;

#[cfg(test)]
mod tests;

pub use hit_target::UiHitTarget;
pub use node::UiNode;
pub use node_children::UiNodeChildren;
pub use node_kind::UiNodeKind;
pub use tree::UiRenderTree;
