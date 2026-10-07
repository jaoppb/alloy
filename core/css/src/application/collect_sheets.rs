//! [`collect_style_sheets`] — the document's own CSS, gathered from the
//! projection (`plano:426`, "`<style>`, `style=` ligados na construção de
//! `StyleSheetSet`").
//!
//! It reads a [`DomSnapshot`], **not** a `dom::DomTree`: `application/snapshot.rs`
//! stays the one file in `core/css` that names a `core/dom` type
//! (`PRD-007:83-84`). A `<style>` element's text children become author-origin
//! rules; each element's `style=` attribute becomes that node's inline block,
//! which outranks every author rule (CSS Cascade L4 §6.4.3) and is therefore
//! carried beside the rule list rather than inside it.
//!
//! `<link rel=stylesheet>` is a subresource and belongs to the network phase
//! (`relatório §2.11`); this function is the seam it will hand its bytes to.

use core::fmt::Write;

use crate::domain::declaration::DeclarationBlock;
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef, SnapshotNodeKind};
use crate::domain::error::CssError;
use crate::domain::stylesheet_set::{Origin, StyleSheetSet};
use crate::infrastructure::parser::{parse_inline_style_recording, parse_stylesheet};

/// Every author rule and inline block the document carries, in document order.
///
/// The `Err` is the one a stylesheet parse raises — hostile nesting
/// (`infrastructure/parser/rules.rs`). Everything else recovered from is a
/// [`crate::ParseNote`] on the returned set.
pub fn collect_style_sheets(snapshot: &DomSnapshot) -> Result<StyleSheetSet, CssError> {
    let mut sheets = StyleSheetSet::new();
    for id in snapshot.nodes_in_document_order() {
        collect_from_node(snapshot, id, &mut sheets)?;
    }
    Ok(sheets)
}

fn collect_from_node(
    snapshot: &DomSnapshot,
    id: crate::domain::dom_snapshot::SnapshotId,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    let Some(node) = snapshot.node(id) else {
        return Ok(());
    };
    collect_presentational_hints(node, sheets)?;
    collect_inline_block(node, sheets)?;
    collect_style_element(snapshot, node, sheets)
}

/// Element presentational hints mapped to author declarations (CSS Cascade L4 §4.2, HTML §15.3.7).
fn collect_presentational_hints(
    node: NodeRef<'_>,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    if node.tag() != Some(&html::TagName::Svg) {
        return Ok(());
    }
    let mut declarations = String::new();
    append_hint_dimension(node.attribute("width"), "width", &mut declarations);
    append_hint_dimension(node.attribute("height"), "height", &mut declarations);
    if declarations.is_empty() {
        return Ok(());
    }
    let block = parse_inline_style_recording(&declarations, sheets)?;
    if !block.is_empty() {
        sheets.push_presentational_hint(node.id(), block);
    }
    Ok(())
}

fn append_hint_dimension(value: Option<&str>, property: &str, target: &mut String) {
    let Some(raw) = value else {
        return;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
        let _ = write!(target, "{property}: {trimmed}px; ");
        return;
    }
    let _ = write!(target, "{property}: {trimmed}; ");
}

/// One element's `style=` attribute.
fn collect_inline_block(node: NodeRef<'_>, sheets: &mut StyleSheetSet) -> Result<(), CssError> {
    let Some(source) = node.attribute(html::AttributeName::style().as_str()) else {
        return Ok(());
    };
    let block = parse_inline_style_recording(source, sheets)?;
    push_inline_block(node, block, sheets);
    Ok(())
}

/// An empty `style=""` is not an inline block — recording one would make every
/// such element carry a rule the cascade then has to skip.
fn push_inline_block(node: NodeRef<'_>, block: DeclarationBlock, sheets: &mut StyleSheetSet) {
    if block.is_empty() {
        return;
    }
    sheets.push_inline(node.id(), block);
}

/// One `<style>` element's text content.
fn collect_style_element(
    snapshot: &DomSnapshot,
    node: NodeRef<'_>,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    if node.tag() != Some(&html::TagName::Style) {
        return Ok(());
    }
    let source = style_element_text(snapshot, node);
    sheets.absorb(parse_stylesheet(&source, Origin::Author)?);
    Ok(())
}

/// The concatenated character data of a `<style>` element's text children. A
/// comment child contributes nothing — CSS inside an HTML comment was already
/// stripped by the tokenizer's `<!--` handling.
fn style_element_text(snapshot: &DomSnapshot, node: NodeRef<'_>) -> String {
    node.children()
        .filter_map(|id| snapshot.node(id))
        .filter(|child| child.kind() == SnapshotNodeKind::Text)
        .filter_map(NodeRef::text)
        .collect()
}
