//! **C-03 / roadmap I1**: a host-owned [`dom::DomTree`] node, readable and
//! mutable from a Rhai script.
//!
//! [`NodeHandle`] is the script-visible type. It carries the tree, a
//! [`dom::NodeId`], and the [`CapabilitySet`] its context was created with —
//! every method checks its own capability before touching the tree, so no path
//! from a script to the DOM skips the guard (C-06), and a missing capability
//! surfaces as [`EngineError::PermissionDenied`] (C-07). Every [`dom::DomError`]
//! is mapped to `EngineError::Subsystem { subsystem: SubsystemName::Dom, .. }`
//! (via [`EngineError::dom`], v0.5 Phase EE) at this boundary; `core/dom` never
//! names an engine type.
//!
//! ## Deviation from v0.2 report §2.5 / §2.7
//!
//! The report specifies `Rc<RefCell<DomTree>>` and an `!Send` `RhaiContext`. The
//! workspace pins `rhai` with the `sync` feature (needed so
//! `RuntimeEngine: Send + Sync`, `PRD-002:35`), which forces every
//! `rhai::CustomType` to be `Send + Sync`. `NodeHandle` therefore holds
//! `Arc<Mutex<DomTree>>`; `RhaiContext` stays `Send + Sync`. A borrow clash
//! (`try_lock` fails) becomes `EngineError::Subsystem { subsystem:
//! SubsystemName::Dom, reason: "DOM is busy", .. }` rather than a
//! `RefCell`/`Mutex` panic — the same safety net the report intends.
//!
//! Reads are exposed as methods (`node.tag()`), not property getters:
//! `rhai::TypeBuilder::with_get` cannot return a `Result`, and a getter that
//! swallowed a type error or a denied capability would violate the error model.

use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use dom::{AttributeName, AttributeValue, DomError, DomTree, NodeId, TagName, TextContent};
use engine::{Capability, CapabilitySet, EngineError, EngineType, TypeRegistration};
use rhai::{Array, CustomType, Dynamic, EvalAltResult, TypeBuilder};

use rhai_runtime::to_eval_error;

/// Which capability each [`NodeHandle`] method requires. The F6 conformance
/// sweep walks this table and asserts every entry is denied when the handle
/// holds no capabilities (C-06).
pub const NODE_HANDLE_BINDINGS: &[(&str, Capability)] = &[
    ("tag", Capability::DOM_READ),
    ("text", Capability::DOM_READ),
    ("children", Capability::DOM_READ),
    ("first_child", Capability::DOM_READ),
    ("last_child", Capability::DOM_READ),
    ("previous_sibling", Capability::DOM_READ),
    ("next_sibling", Capability::DOM_READ),
    ("parent", Capability::DOM_READ),
    ("get_attribute", Capability::DOM_READ),
    ("create_element", Capability::DOM_MUTATE),
    ("create_text", Capability::DOM_MUTATE),
    ("append_child", Capability::DOM_MUTATE),
    ("set_text", Capability::DOM_MUTATE),
    ("set_attribute", Capability::DOM_MUTATE),
    ("remove_attribute", Capability::DOM_MUTATE),
];

/// A script-held reference to one node of a bound [`DomTree`]. Cloning is an
/// `Arc` bump plus two `Copy` fields.
#[derive(Clone)]
pub struct NodeHandle {
    tree: Arc<Mutex<DomTree>>,
    id: NodeId,
    capabilities: CapabilitySet,
}

impl NodeHandle {
    pub(crate) const fn new(
        tree: Arc<Mutex<DomTree>>,
        id: NodeId,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            tree,
            id,
            capabilities,
        }
    }

    fn sibling(&self, id: NodeId) -> Self {
        Self {
            tree: Arc::clone(&self.tree),
            id,
            capabilities: self.capabilities,
        }
    }

    fn require(&self, capability: Capability) -> Result<(), Box<EvalAltResult>> {
        self.capabilities.require(capability).map_err(to_eval_error)
    }

    fn lock(&self, operation: &str) -> Result<MutexGuard<'_, DomTree>, Box<EvalAltResult>> {
        match self.tree.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::Poisoned(poison)) => Ok(poison.into_inner()),
            Err(TryLockError::WouldBlock) => {
                Err(to_eval_error(EngineError::dom(operation, "DOM is busy")))
            }
        }
    }

    fn tag(&self) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("tag")?;
        tree.tag(self.id)
            .map(|tag| tag.as_str().to_owned())
            .map_err(|error| dom_error("tag", &error))
    }

    fn text(&self) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("text")?;
        tree.text(self.id)
            .map(|text| text.as_str().to_owned())
            .map_err(|error| dom_error("text", &error))
    }

    fn children(&self) -> Result<Array, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let ids: Vec<NodeId> = {
            let tree = self.lock("children")?;
            tree.children(self.id).collect()
        };
        Ok(ids
            .into_iter()
            .map(|child| Dynamic::from(self.sibling(child)))
            .collect())
    }

    fn first_child(&self) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("first_child")?;
        let id = tree
            .first_child(self.id)
            .map_err(|error| dom_error("first_child", &error))?;
        drop(tree);
        Ok(id.map_or(Dynamic::UNIT, |child| Dynamic::from(self.sibling(child))))
    }

    fn last_child(&self) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("last_child")?;
        let id = tree
            .last_child(self.id)
            .map_err(|error| dom_error("last_child", &error))?;
        drop(tree);
        Ok(id.map_or(Dynamic::UNIT, |child| Dynamic::from(self.sibling(child))))
    }

    fn previous_sibling(&self) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("previous_sibling")?;
        let id = tree
            .previous_sibling(self.id)
            .map_err(|error| dom_error("previous_sibling", &error))?;
        drop(tree);
        Ok(id.map_or(Dynamic::UNIT, |sibling| {
            Dynamic::from(self.sibling(sibling))
        }))
    }

    fn next_sibling(&self) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("next_sibling")?;
        let id = tree
            .next_sibling(self.id)
            .map_err(|error| dom_error("next_sibling", &error))?;
        drop(tree);
        Ok(id.map_or(Dynamic::UNIT, |sibling| {
            Dynamic::from(self.sibling(sibling))
        }))
    }

    fn parent(&self) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree = self.lock("parent")?;
        let id = tree
            .parent(self.id)
            .map_err(|error| dom_error("parent", &error))?;
        drop(tree);
        Ok(id.map_or(Dynamic::UNIT, |parent| Dynamic::from(self.sibling(parent))))
    }

    fn get_attribute(&self, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let attribute_name =
            AttributeName::new(name).map_err(|error| dom_error("get_attribute", &error))?;
        let tree = self.lock("get_attribute")?;
        let value = tree
            .attribute(self.id, &attribute_name)
            .map_err(|error| dom_error("get_attribute", &error))?;
        let result = value.map_or(Dynamic::UNIT, |found| {
            Dynamic::from(found.as_str().to_owned())
        });
        drop(tree);
        Ok(result)
    }

    fn create_element(&self, tag: &str) -> Result<Self, Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        let tag_name = TagName::new(tag).map_err(|error| dom_error("create_element", &error))?;
        let created = self.lock("create_element")?.create_element(tag_name);
        Ok(self.sibling(created))
    }

    fn create_text(&self, content: &str) -> Result<Self, Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        let created = self
            .lock("create_text")?
            .create_text(TextContent::new(content));
        Ok(self.sibling(created))
    }

    fn append_child(&self, child: &Self) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        if !Arc::ptr_eq(&self.tree, &child.tree) {
            return Err(to_eval_error(EngineError::dom(
                "append_child",
                "node belongs to another document",
            )));
        }
        let mut tree = self.lock("append_child")?;
        tree.append_child(self.id, child.id)
            .map_err(|error| dom_error("append_child", &error))
    }

    fn set_text(&self, content: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        let mut tree = self.lock("set_text")?;
        tree.set_text(self.id, TextContent::new(content))
            .map_err(|error| dom_error("set_text", &error))
    }

    fn set_attribute(&self, name: &str, value: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        let attribute_name =
            AttributeName::new(name).map_err(|error| dom_error("set_attribute", &error))?;
        let mut tree = self.lock("set_attribute")?;
        tree.set_attribute(self.id, attribute_name, AttributeValue::new(value))
            .map_err(|error| dom_error("set_attribute", &error))
    }

    fn remove_attribute(&self, name: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::DOM_MUTATE)?;
        let attribute_name =
            AttributeName::new(name).map_err(|error| dom_error("remove_attribute", &error))?;
        let mut tree = self.lock("remove_attribute")?;
        tree.remove_attribute(self.id, &attribute_name)
            .map_err(|error| dom_error("remove_attribute", &error))
    }
}

#[allow(clippy::unnecessary_box_returns)]
fn dom_error(operation: &str, error: &DomError) -> Box<EvalAltResult> {
    to_eval_error(EngineError::dom(operation, error.to_string()))
}

impl EngineType for NodeHandle {
    fn registration() -> TypeRegistration {
        TypeRegistration::new("Node")
    }
}

impl CustomType for NodeHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("Node")
            .with_fn("tag", |handle: &mut Self| handle.tag())
            .with_fn("text", |handle: &mut Self| handle.text())
            .with_fn("children", |handle: &mut Self| handle.children())
            .with_fn("first_child", |handle: &mut Self| handle.first_child())
            .with_fn("last_child", |handle: &mut Self| handle.last_child())
            .with_fn("previous_sibling", |handle: &mut Self| {
                handle.previous_sibling()
            })
            .with_fn("next_sibling", |handle: &mut Self| handle.next_sibling())
            .with_fn("parent", |handle: &mut Self| handle.parent())
            .with_fn("get_attribute", |handle: &mut Self, name: &str| {
                handle.get_attribute(name)
            })
            .with_fn("create_element", |handle: &mut Self, tag: &str| {
                handle.create_element(tag)
            })
            .with_fn("create_text", |handle: &mut Self, content: &str| {
                handle.create_text(content)
            })
            .with_fn("append_child", |handle: &mut Self, child: Self| {
                handle.append_child(&child)
            })
            .with_fn("set_text", |handle: &mut Self, content: &str| {
                handle.set_text(content)
            })
            .with_fn(
                "set_attribute",
                |handle: &mut Self, name: &str, value: &str| handle.set_attribute(name, value),
            )
            .with_fn("remove_attribute", |handle: &mut Self, name: &str| {
                handle.remove_attribute(name)
            });
    }
}
