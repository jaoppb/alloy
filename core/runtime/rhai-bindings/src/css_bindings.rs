//! CSS bindings for Rhai scripts (Fase M, PRD-007 §3.4, ADR-0003, ADR-0010, ADR-0011).
//!
//! Exposes [`DomSnapshot`] and [`StyledTree`] as script types
//! ([`SnapshotHandle`] and [`StyledTreeHandle`]) under [`Capability::DOM_READ`]
//! and [`Capability::GRAPHICS_DRAW`], strictly without [`Capability::DOM_MUTATE`].
//! Implements [`ScriptCascadeResolver`] with automated 3-tier fallback to [`UaCascade`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use css::{
    CascadeResolver, ComputedStyle, CssColor, CssError, Display, DomSnapshot, SnapshotId,
    SnapshotNodeKind, StyleSheetSet, StyledTree, UaCascade,
};
use engine::{
    Capability, CapabilitySet, EngineError, EngineType, EngineValue, RuntimeEngine, SubsystemName,
    TypeRegistration, VariableName, profiles,
};
use rhai::{Array, CustomType, Dynamic, EvalAltResult, TypeBuilder};
use rhai_runtime::{
    PanicHookGuard, RhaiCompiledScript, RhaiContext, RhaiEngine, run_with_fallback, to_eval_error,
};

use crate::DEFAULT_CASCADE_SCRIPT;

#[allow(clippy::unnecessary_box_returns)]
fn css_error(operation: &str, error: impl core::fmt::Display) -> Box<EvalAltResult> {
    to_eval_error(EngineError::subsystem(
        SubsystemName::Css,
        operation,
        error.to_string(),
    ))
}

/// A read-only handle to a [`DomSnapshot`] inside a Rhai context.
#[derive(Clone)]
pub struct SnapshotHandle {
    snapshot: Arc<DomSnapshot>,
    node_ids: Arc<Vec<SnapshotId>>,
    capabilities: CapabilitySet,
}

impl SnapshotHandle {
    #[must_use]
    pub fn new(snapshot: Arc<DomSnapshot>, capabilities: CapabilitySet) -> Self {
        let node_ids = Arc::new(snapshot.nodes_in_document_order().collect());
        Self {
            snapshot,
            node_ids,
            capabilities,
        }
    }

    fn require(&self, capability: Capability) -> Result<(), Box<EvalAltResult>> {
        self.capabilities.require(capability).map_err(to_eval_error)
    }

    fn id_from_index(&self, index: i64) -> Result<SnapshotId, Box<EvalAltResult>> {
        if index < 0 {
            return Err(css_error("id_from_index", "negative node index"));
        }
        let unsigned_index = usize::try_from(index)
            .map_err(|error| css_error("id_from_index", error.to_string()))?;
        self.node_ids
            .get(unsigned_index)
            .copied()
            .ok_or_else(|| css_error("id_from_index", "node index out of bounds"))
    }

    fn root(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let root_id = self.snapshot.root();
        let root_index = root_id.index();
        Ok(i64::try_from(root_index).unwrap_or(0))
    }

    fn len(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let snapshot_len = self.snapshot.len();
        Ok(i64::try_from(snapshot_len).unwrap_or(0))
    }

    fn tag(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("tag", "invalid node id"))?;
        let tag_name = node_ref.tag_str().unwrap_or("");
        Ok(tag_name.to_owned())
    }

    fn attribute(&self, node_index: i64, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("attribute", "invalid node id"))?;
        let attribute_value = node_ref.attribute(name);
        Ok(attribute_value.map_or(Dynamic::UNIT, |value| Dynamic::from(value.to_owned())))
    }

    fn children(&self, node_index: i64) -> Result<Array, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("children", "invalid node id"))?;
        let children_array: Vec<Dynamic> = node_ref
            .children()
            .map(|child_id| Dynamic::from(i64::try_from(child_id.index()).unwrap_or(0)))
            .collect();
        Ok(children_array)
    }

    fn parent(&self, node_index: i64) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("parent", "invalid node id"))?;
        let parent_index = node_ref
            .parent()
            .map(|parent_id| i64::try_from(parent_id.index()).unwrap_or(0));
        Ok(parent_index.map_or(Dynamic::UNIT, Dynamic::from))
    }

    fn kind(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("kind", "invalid node id"))?;
        let kind_text = match node_ref.kind() {
            SnapshotNodeKind::Document => "document",
            SnapshotNodeKind::Element => "element",
            SnapshotNodeKind::Text => "text",
            SnapshotNodeKind::Comment => "comment",
            _ => "unknown",
        };
        Ok(kind_text.to_owned())
    }

    fn text(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(node_id)
            .ok_or_else(|| css_error("text", "invalid node id"))?;
        let text_content = node_ref.text().unwrap_or("");
        Ok(text_content.to_owned())
    }
}

impl EngineType for SnapshotHandle {
    fn registration() -> TypeRegistration {
        TypeRegistration::new("DomSnapshot")
    }
}

impl CustomType for SnapshotHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("DomSnapshot")
            .with_fn("root", |handle: &mut Self| handle.root())
            .with_fn("len", |handle: &mut Self| handle.len())
            .with_fn("tag", |handle: &mut Self, id: i64| handle.tag(id))
            .with_fn("attribute", |handle: &mut Self, id: i64, name: &str| {
                handle.attribute(id, name)
            })
            .with_fn("children", |handle: &mut Self, id: i64| handle.children(id))
            .with_fn("parent", |handle: &mut Self, id: i64| handle.parent(id))
            .with_fn("kind", |handle: &mut Self, id: i64| handle.kind(id))
            .with_fn("text", |handle: &mut Self, id: i64| handle.text(id));
    }
}

/// A handle to a [`StyledTree`] inside a Rhai context.
///
/// Can read computed styles under [`Capability::DOM_READ`], and record style overrides
/// under [`Capability::GRAPHICS_DRAW`]. Strictly does not allow DOM mutation.
#[derive(Clone)]
pub struct StyledTreeHandle {
    base: Arc<StyledTree>,
    node_ids: Arc<Vec<SnapshotId>>,
    overrides: Arc<Mutex<HashMap<usize, ComputedStyle>>>,
    capabilities: CapabilitySet,
}

impl StyledTreeHandle {
    #[must_use]
    pub fn new(base: Arc<StyledTree>, capabilities: CapabilitySet) -> Self {
        let node_ids = Arc::new(
            base.nodes_in_document_order()
                .map(css::StyledNode::node)
                .collect(),
        );
        Self {
            base,
            node_ids,
            overrides: Arc::new(Mutex::new(HashMap::new())),
            capabilities,
        }
    }

    fn require(&self, capability: Capability) -> Result<(), Box<EvalAltResult>> {
        self.capabilities.require(capability).map_err(to_eval_error)
    }

    fn id_from_index(&self, index: i64) -> Result<SnapshotId, Box<EvalAltResult>> {
        if index < 0 {
            return Err(css_error("id_from_index", "negative node index"));
        }
        let unsigned_index = usize::try_from(index)
            .map_err(|error| css_error("id_from_index", error.to_string()))?;
        self.node_ids
            .get(unsigned_index)
            .copied()
            .ok_or_else(|| css_error("id_from_index", "node index out of bounds"))
    }

    fn root(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let root_id = self.base.root();
        let root_index = root_id.index();
        Ok(i64::try_from(root_index).unwrap_or(0))
    }

    fn len(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let tree_len = self.base.len();
        Ok(i64::try_from(tree_len).unwrap_or(0))
    }

    fn current_style(&self, node_id: SnapshotId) -> Result<ComputedStyle, Box<EvalAltResult>> {
        let from_overrides = {
            let guard = self
                .overrides
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.get(&node_id.index()).copied()
        };
        if let Some(existing) = from_overrides {
            return Ok(existing);
        }
        let styled = self
            .base
            .node(node_id)
            .ok_or_else(|| css_error("current_style", "styled node not found"))?;
        Ok(*styled.style())
    }

    fn color(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let style = self.current_style(node_id)?;
        let color_string = style.color().to_string();
        Ok(color_string)
    }

    fn background_color(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let style = self.current_style(node_id)?;
        let bg_string = style.background_color().to_string();
        Ok(bg_string)
    }

    fn display(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let node_id = self.id_from_index(node_index)?;
        let style = self.current_style(node_id)?;
        let keyword_text = style.display().keyword().to_owned();
        Ok(keyword_text)
    }

    fn set_color(&self, node_index: i64, color_text: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let node_id = self.id_from_index(node_index)?;
        let color = color_text
            .parse::<CssColor>()
            .map_err(|error| css_error("set_color", error))?;
        let current = self.current_style(node_id)?;
        let updated = current.with_color(color);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(node_id.index(), updated);
        Ok(())
    }

    fn set_background_color(
        &self,
        node_index: i64,
        color_text: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let node_id = self.id_from_index(node_index)?;
        let color = color_text
            .parse::<CssColor>()
            .map_err(|error| css_error("set_background_color", error))?;
        let current = self.current_style(node_id)?;
        let updated = current.with_background_color(color);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(node_id.index(), updated);
        Ok(())
    }

    fn set_display(&self, node_index: i64, display_text: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let node_id = self.id_from_index(node_index)?;
        let display = display_text
            .parse::<Display>()
            .map_err(|error| css_error("set_display", error))?;
        let current = self.current_style(node_id)?;
        let updated = current.with_display(display);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(node_id.index(), updated);
        Ok(())
    }
}

impl EngineType for StyledTreeHandle {
    fn registration() -> TypeRegistration {
        TypeRegistration::new("StyledTree")
    }
}

impl CustomType for StyledTreeHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("StyledTree")
            .with_fn("root", |handle: &mut Self| handle.root())
            .with_fn("len", |handle: &mut Self| handle.len())
            .with_fn("color", |handle: &mut Self, id: i64| handle.color(id))
            .with_fn("background_color", |handle: &mut Self, id: i64| {
                handle.background_color(id)
            })
            .with_fn("display", |handle: &mut Self, id: i64| handle.display(id))
            .with_fn("set_color", |handle: &mut Self, id: i64, color: &str| {
                handle.set_color(id, color)
            })
            .with_fn(
                "set_background_color",
                |handle: &mut Self, id: i64, color: &str| handle.set_background_color(id, color),
            )
            .with_fn(
                "set_display",
                |handle: &mut Self, id: i64, display: &str| handle.set_display(id, display),
            );
    }
}

/// Register CSS types on a Rhai context.
pub fn register_css_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    context.register_custom_type::<SnapshotHandle>()?;
    context.register_custom_type::<StyledTreeHandle>()?;
    Ok(())
}

/// A scriptable cascade resolver executing `.rhai` under [`profiles::css_cascade`].
///
/// Under C-09 and PRD-007 §3.4: falls back automatically via 3-tier fallback to
/// [`DEFAULT_CASCADE_SCRIPT`] and [`UaCascade`] whenever a script compilation,
/// evaluation, limit, or panic occurs.
pub struct ScriptCascadeResolver {
    engine: RhaiEngine,
    primary_script: Option<RhaiCompiledScript>,
    default_script: Option<RhaiCompiledScript>,
    fallback: UaCascade,
}

impl ScriptCascadeResolver {
    /// Create a new resolver with the given Rhai engine and script source.
    #[must_use]
    pub fn new(engine: RhaiEngine, script_source: impl Into<String>) -> Self {
        let script_text = script_source.into();
        let primary_script = engine.compile(&script_text).ok();
        let default_script = engine.compile(DEFAULT_CASCADE_SCRIPT).ok();
        Self {
            engine,
            primary_script,
            default_script,
            fallback: UaCascade::new(),
        }
    }

    fn evaluate_cascade_script(
        &self,
        dom: &DomSnapshot,
        sheets: &StyleSheetSet,
        compiled: Option<&RhaiCompiledScript>,
    ) -> Result<(StyledTree, EngineValue), EngineError> {
        let Some(script) = compiled else {
            return Err(EngineError::subsystem(
                SubsystemName::Css,
                "resolve",
                "script compilation failed",
            ));
        };

        let base_tree = self.fallback.resolve(dom, sheets).map_err(|error| {
            EngineError::subsystem(SubsystemName::Css, "resolve", error.to_string())
        })?;

        let capabilities = profiles::css_cascade();
        let mut context = self.engine.create_context(capabilities)?;
        register_css_bindings(&mut context)?;

        let snapshot_handle = SnapshotHandle::new(Arc::new(dom.clone()), capabilities);
        let styled_handle = StyledTreeHandle::new(Arc::new(base_tree.clone()), capabilities);
        let overrides = Arc::clone(&styled_handle.overrides);

        let dom_var = VariableName::parse("dom")?;
        let tree_var = VariableName::parse("tree")?;
        context.set_custom_value(&dom_var, snapshot_handle);
        context.set_custom_value(&tree_var, styled_handle);

        let outcome = {
            let _quiet = PanicHookGuard::install();
            self.engine.eval_compiled_value(&mut context, script)?
        };

        let overrides_map = {
            let guard = overrides.lock().unwrap_or_else(PoisonError::into_inner);
            guard.clone()
        };
        if overrides_map.is_empty() {
            return Ok((base_tree, outcome));
        }

        let recomputed = StyledTree::recompute_in_document_order(dom, |node_ref, parent_style| {
            let index = node_ref.id().index();
            if let Some(override_style) = overrides_map.get(&index) {
                return *override_style;
            }
            if let Some(styled_node) = base_tree.node(node_ref.id()) {
                return *styled_node.style();
            }
            if let Some(parent) = parent_style {
                return ComputedStyle::inheriting_from(parent);
            }
            ComputedStyle::initial()
        });

        Ok((recomputed, outcome))
    }
}

impl CascadeResolver for ScriptCascadeResolver {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        let (tree, _) = run_with_fallback(
            None,
            || self.evaluate_cascade_script(dom, sheets, self.primary_script.as_ref()),
            || {
                self.evaluate_cascade_script(dom, sheets, self.default_script.as_ref())
                    .map(|(styled_tree, _)| styled_tree)
            },
            || {
                self.fallback.resolve(dom, sheets).unwrap_or_else(|_| {
                    StyledTree::recompute_in_document_order(dom, |_node_ref, _parent| {
                        ComputedStyle::initial()
                    })
                })
            },
        );
        Ok(tree)
    }
}
