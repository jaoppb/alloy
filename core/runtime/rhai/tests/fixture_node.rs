//! **C-02** (`PRD-002:89`): a registered Rust domain struct is readable and
//! mutable from a Rhai script.
//!
//! `FixtureNode` is a stand-in for the real `core/dom` `DomNode`, which arrives
//! at roadmap I1 (v0.2). C-03 stays open until then.

use engine::{
    Capability, CapabilitySet, EngineType, RuntimeEngine, TypeRegistration, VariableName,
};
use rhai::{CustomType, TypeBuilder};
use rhai_runtime::RhaiEngine;

#[derive(Clone)]
struct FixtureNode {
    id: i64,
    tag: String,
    text: String,
}

impl FixtureNode {
    fn new(id: i64, tag: &str, text: &str) -> Self {
        Self {
            id,
            tag: tag.to_owned(),
            text: text.to_owned(),
        }
    }
}

impl EngineType for FixtureNode {
    fn registration() -> TypeRegistration {
        TypeRegistration::new("FixtureNode")
    }
}

impl CustomType for FixtureNode {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("FixtureNode")
            .with_get("id", |node: &mut Self| node.id)
            .with_get("tag", |node: &mut Self| node.tag.clone())
            .with_get_set(
                "text",
                |node: &mut Self| node.text.clone(),
                |node: &mut Self, value: String| node.text = value,
            );
    }
}

#[test]
fn a_registered_domain_struct_is_readable_and_mutable_from_script() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::new(
            Capability::DOM_READ | Capability::DOM_MUTATE,
        ))
        .expect("context");
    context
        .register_custom_type::<FixtureNode>()
        .expect("register FixtureNode");
    let node_name = VariableName::parse("node").expect("valid name");
    context.set_custom_value(&node_name, FixtureNode::new(7, "div", "hello"));

    // read
    let tag: String = engine
        .eval(&mut context, "node.tag")
        .expect("read node.tag");
    assert_eq!(tag, "div");

    // mutate
    engine
        .eval_value(&mut context, r#"node.text = node.text + " world""#)
        .expect("mutate node.text from script");

    let mutated: FixtureNode = context
        .custom_value(&node_name)
        .expect("read node back into Rust");
    assert_eq!(
        mutated.text, "hello world",
        "the script's mutation is visible in Rust"
    );
    assert_eq!(mutated.id, 7, "an unrelated field is untouched");

    assert_eq!(context.registered_type_names(), ["FixtureNode"]);
}
