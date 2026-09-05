//! CSS bindings for Rhai scripts (Fase M, PRD-007 §3.4, ADR-0003, ADR-0011).
//!
//! Exposes [`DomSnapshot`] and [`StyledTree`] as read-only script types
//! ([`SnapshotHandle`] and [`StyledTreeHandle`]) under [`Capability::DOM_READ`]
//! and [`Capability::GRAPHICS_DRAW`], strictly without [`Capability::DOM_MUTATE`].
//! Implements [`ScriptCascadeResolver`] with automated fallback to [`UaCascade`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use css::{
    CascadeResolver, ComputedStyle, CssColor, CssError, Display, DomSnapshot, SnapshotId,
    SnapshotNodeKind, StyleSheetSet, StyledTree, UaCascade,
};
use engine::{
    Capability, CapabilitySet, EngineError, EngineType, RuntimeEngine, SubsystemName,
    TypeRegistration, VariableName, profiles,
};
use rhai::{Array, CustomType, Dynamic, EvalAltResult, TypeBuilder};
use rhai_runtime::{PanicHookGuard, RhaiContext, RhaiEngine, to_eval_error};

#[allow(clippy::unnecessary_box_returns)]
fn css_error(operation: &str, error_message: impl Into<String>) -> Box<EvalAltResult> {
    to_eval_error(EngineError::subsystem(
        SubsystemName::Css,
        operation,
        error_message,
    ))
}

/// A read-only handle to a [`DomSnapshot`] inside a Rhai context.
#[derive(Clone)]
pub struct SnapshotHandle {
    snapshot: Arc<DomSnapshot>,
    capabilities: CapabilitySet,
}

impl SnapshotHandle {
    #[must_use]
    pub const fn new(snapshot: Arc<DomSnapshot>, capabilities: CapabilitySet) -> Self {
        Self {
            snapshot,
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
        let unsigned = usize::try_from(index)
            .map_err(|error| css_error("id_from_index", format!("{error}")))?;
        self.snapshot
            .nodes_in_document_order()
            .nth(unsigned)
            .ok_or_else(|| css_error("id_from_index", "node index out of bounds"))
    }

    fn root(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        Ok(i64::try_from(self.snapshot.root().index()).unwrap_or(0))
    }

    fn len(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        Ok(i64::try_from(self.snapshot.len()).unwrap_or(0))
    }

    fn tag(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
            .ok_or_else(|| css_error("tag", "invalid node id"))?;
        Ok(node_ref.tag().unwrap_or("").to_owned())
    }

    fn attribute(&self, node_index: i64, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
            .ok_or_else(|| css_error("attribute", "invalid node id"))?;
        let value = node_ref.attribute(name);
        Ok(value.map_or(Dynamic::UNIT, |val| Dynamic::from(val.to_owned())))
    }

    fn children(&self, node_index: i64) -> Result<Array, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
            .ok_or_else(|| css_error("children", "invalid node id"))?;
        let children: Vec<Dynamic> = node_ref
            .children()
            .map(|child_id| Dynamic::from(i64::try_from(child_id.index()).unwrap_or(0)))
            .collect();
        Ok(children)
    }

    fn parent(&self, node_index: i64) -> Result<Dynamic, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
            .ok_or_else(|| css_error("parent", "invalid node id"))?;
        let parent = node_ref
            .parent()
            .map(|parent_id| i64::try_from(parent_id.index()).unwrap_or(0));
        Ok(parent.map_or(Dynamic::UNIT, Dynamic::from))
    }

    fn kind(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
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
        let id = self.id_from_index(node_index)?;
        let node_ref = self
            .snapshot
            .node(id)
            .ok_or_else(|| css_error("text", "invalid node id"))?;
        Ok(node_ref.text().unwrap_or("").to_owned())
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
    overrides: Arc<Mutex<HashMap<usize, ComputedStyle>>>,
    capabilities: CapabilitySet,
}

impl StyledTreeHandle {
    #[must_use]
    pub fn new(base: Arc<StyledTree>, capabilities: CapabilitySet) -> Self {
        Self {
            base,
            overrides: Arc::new(Mutex::new(HashMap::new())),
            capabilities,
        }
    }

    fn require(&self, capability: Capability) -> Result<(), Box<EvalAltResult>> {
        self.capabilities.require(capability).map_err(to_eval_error)
    }

    fn root(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        Ok(i64::try_from(self.base.root().index()).unwrap_or(0))
    }

    fn len(&self) -> Result<i64, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        Ok(i64::try_from(self.base.len()).unwrap_or(0))
    }

    fn current_style(&self, node_index: usize) -> Result<ComputedStyle, Box<EvalAltResult>> {
        let from_overrides = {
            let guard = self
                .overrides
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            guard.get(&node_index).copied()
        };
        if let Some(existing) = from_overrides {
            return Ok(existing);
        }
        let id = self
            .base
            .nodes_in_document_order()
            .nth(node_index)
            .map(css::StyledNode::node)
            .ok_or_else(|| css_error("current_style", "node index out of bounds"))?;
        let styled = self
            .base
            .node(id)
            .ok_or_else(|| css_error("current_style", "styled node not found"))?;
        Ok(*styled.style())
    }

    fn color(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let unsigned =
            usize::try_from(node_index).map_err(|error| css_error("color", format!("{error}")))?;
        let style = self.current_style(unsigned)?;
        Ok(style.color().to_string())
    }

    fn background_color(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let unsigned = usize::try_from(node_index)
            .map_err(|error| css_error("background_color", format!("{error}")))?;
        let style = self.current_style(unsigned)?;
        Ok(style.background_color().to_string())
    }

    fn display(&self, node_index: i64) -> Result<String, Box<EvalAltResult>> {
        self.require(Capability::DOM_READ)?;
        let unsigned = usize::try_from(node_index)
            .map_err(|error| css_error("display", format!("{error}")))?;
        let style = self.current_style(unsigned)?;
        Ok(style.display().keyword().to_owned())
    }

    fn set_color(&self, node_index: i64, color_text: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let unsigned = usize::try_from(node_index)
            .map_err(|error| css_error("set_color", format!("{error}")))?;
        let color = parse_css_color(color_text).map_err(|error| css_error("set_color", error))?;
        let current = self.current_style(unsigned)?;
        let updated = current.with_color(color);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(unsigned, updated);
        Ok(())
    }

    fn set_background_color(
        &self,
        node_index: i64,
        color_text: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let unsigned = usize::try_from(node_index)
            .map_err(|error| css_error("set_background_color", format!("{error}")))?;
        let color = parse_css_color(color_text)
            .map_err(|error| css_error("set_background_color", error))?;
        let current = self.current_style(unsigned)?;
        let updated = current.with_background_color(color);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(unsigned, updated);
        Ok(())
    }

    fn set_display(&self, node_index: i64, display_text: &str) -> Result<(), Box<EvalAltResult>> {
        self.require(Capability::GRAPHICS_DRAW)?;
        let unsigned = usize::try_from(node_index)
            .map_err(|error| css_error("set_display", format!("{error}")))?;
        let display = match display_text {
            "none" => Display::None,
            "block" => Display::Block,
            "inline" => Display::Inline,
            "flex" => Display::Flex,
            other => {
                return Err(css_error(
                    "set_display",
                    format!("unsupported display keyword `{other}`"),
                ));
            }
        };
        let current = self.current_style(unsigned)?;
        let updated = current.with_display(display);
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(unsigned, updated);
        Ok(())
    }
}

fn parse_css_color(text: &str) -> Result<CssColor, String> {
    let lower = text.trim().to_ascii_lowercase();
    match lower.as_str() {
        "black" => Ok(CssColor::BLACK),
        "transparent" => Ok(CssColor::TRANSPARENT),
        "white" => Ok(CssColor::rgb(255, 255, 255)),
        "red" => Ok(CssColor::rgb(255, 0, 0)),
        "green" => Ok(CssColor::rgb(0, 128, 0)),
        "blue" => Ok(CssColor::rgb(0, 0, 255)),
        _ => parse_hex_color(&lower),
    }
}

fn parse_hex_color(text: &str) -> Result<CssColor, String> {
    let hex_part = text
        .strip_prefix('#')
        .ok_or_else(|| format!("unrecognised colour literal `{text}`"))?;
    if hex_part.len() == 6 {
        let red = u8::from_str_radix(hex_part.get(0..2).unwrap_or("00"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        let green = u8::from_str_radix(hex_part.get(2..4).unwrap_or("00"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        let blue = u8::from_str_radix(hex_part.get(4..6).unwrap_or("00"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        return Ok(CssColor::rgb(red, green, blue));
    }
    if hex_part.len() == 3 {
        let red_char = hex_part.chars().next().unwrap_or('0');
        let green_char = hex_part.chars().nth(1).unwrap_or('0');
        let blue_char = hex_part.chars().nth(2).unwrap_or('0');
        let red = u8::from_str_radix(&format!("{red_char}{red_char}"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        let green = u8::from_str_radix(&format!("{green_char}{green_char}"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        let blue = u8::from_str_radix(&format!("{blue_char}{blue_char}"), 16)
            .map_err(|error| format!("invalid hex colour: {error}"))?;
        return Ok(CssColor::rgb(red, green, blue));
    }
    Err(format!("unsupported hex colour length in `{text}`"))
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
/// Under C-09 and PRD-007 §3.4: falls back automatically to [`UaCascade`]
/// whenever a script compilation, evaluation, limit, or panic occurs.
pub struct ScriptCascadeResolver {
    engine: RhaiEngine,
    script: String,
    fallback: UaCascade,
}

impl ScriptCascadeResolver {
    /// Create a new resolver with the given Rhai engine and script source.
    #[must_use]
    pub fn new(engine: RhaiEngine, script: impl Into<String>) -> Self {
        Self {
            engine,
            script: script.into(),
            fallback: UaCascade::new(),
        }
    }

    fn resolve_with_script(
        &self,
        dom: &DomSnapshot,
        sheets: &StyleSheetSet,
    ) -> Result<StyledTree, EngineError> {
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

        {
            let _quiet = PanicHookGuard::install();
            self.engine.eval_value(&mut context, &self.script)?;
        }

        let overrides_map = {
            let guard = overrides.lock().unwrap_or_else(PoisonError::into_inner);
            guard.clone()
        };
        if overrides_map.is_empty() {
            return Ok(base_tree);
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

        Ok(recomputed)
    }
}

impl CascadeResolver for ScriptCascadeResolver {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError> {
        match self.resolve_with_script(dom, sheets) {
            Ok(tree) => Ok(tree),
            Err(error) => {
                tracing::warn!("script cascade failed ({error}); using UaCascade fallback");
                self.fallback.resolve(dom, sheets)
            }
        }
    }
}
