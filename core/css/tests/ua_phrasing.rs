//! The phrasing-content rules of the embedded UA sheet (WHATWG HTML §15.3.4,
//! issue #133 of #81): the computed values a page gets with no author CSS at
//! all, through the whole chain `dom::parse → snapshot → collect_style_sheets
//! → UaCascade::resolve`. Painting them is #57's; this file pins the cascade.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::domain::computed::text_advance::{FontStyle, FontWeight, TextDecoration};
use css::{
    CascadeResolver, ComputedStyle, CssColor, Display, FontFamily, GenericFamily, UaCascade,
    collect_style_sheets, snapshot,
};

/// The computed style of the element carrying `id="target"` in `body`.
fn style_of(body: &str) -> ComputedStyle {
    let markup = format!("<!doctype html><html><body>{body}</body></html>");
    let tree = dom::parse(&markup).expect("the fixture parses").into_tree();
    let dom = snapshot(&tree, tree.document());
    let sheets = collect_style_sheets(&dom).expect("the fixture has no unreadable CSS");
    let styled = UaCascade::new()
        .resolve(&dom, &sheets)
        .expect("the cascade resolves");
    let target = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(|node| node.attribute("id")) == Some("target"))
        .expect("the fixture has a #target");
    styled
        .node(target)
        .expect("#target is styled")
        .style()
        .clone()
}

fn decoration_of(body: &str) -> TextDecoration {
    style_of(body).text_advance().text_decoration()
}

#[test]
fn emphasis_elements_are_italic() {
    for tag in ["em", "i", "cite", "dfn", "var", "address"] {
        let style = style_of(&format!("<{tag} id=target>x</{tag}>"));
        assert_eq!(
            style.text_advance().font_style(),
            FontStyle::Italic,
            "<{tag}>"
        );
    }
}

#[test]
fn strong_importance_is_one_step_bolder_than_its_parent() {
    for tag in ["strong", "b"] {
        let style = style_of(&format!("<{tag} id=target>x</{tag}>"));
        assert_eq!(
            style.text_advance().font_weight(),
            FontWeight::BOLD,
            "<{tag}>"
        );
    }
    let nested = style_of("<strong><strong id=target>x</strong></strong>");
    assert_eq!(
        nested.text_advance().font_weight(),
        FontWeight::BLACK,
        "`bolder` is relative to the parent's weight (CSS Fonts 4 §2.2)"
    );
}

#[test]
fn code_like_elements_are_monospace() {
    for tag in ["code", "kbd", "samp", "tt", "pre", "listing", "xmp"] {
        let style = style_of(&format!("<{tag} id=target>x</{tag}>"));
        assert_eq!(
            style.font_family().primary(),
            Some(FontFamily::Generic(GenericFamily::Monospace)),
            "<{tag}>"
        );
    }
}

#[test]
fn a_link_is_blue_and_underlined() {
    let style = style_of("<a id=target href=x>x</a>");
    assert_eq!(style.color(), CssColor::rgb(0x00, 0x00, 0xEE));
    assert!(
        style
            .text_advance()
            .text_decoration()
            .line()
            .has_underline()
    );
}

#[test]
fn an_anchor_without_href_is_not_a_link() {
    let style = style_of("<a id=target name=x>x</a>");
    assert_eq!(style.color(), CssColor::BLACK, "`:link` needs an `href`");
    assert!(style.text_advance().text_decoration().line().is_none());
}

#[test]
fn edits_and_underlines_carry_their_line() {
    for tag in ["u", "ins"] {
        let line = decoration_of(&format!("<{tag} id=target>x</{tag}>")).line();
        assert!(line.has_underline(), "<{tag}>");
    }
    for tag in ["del", "s", "strike"] {
        let line = decoration_of(&format!("<{tag} id=target>x</{tag}>")).line();
        assert!(line.has_line_through(), "<{tag}>");
    }
}

#[test]
fn mark_is_black_on_yellow() {
    let style = style_of("<mark id=target>x</mark>");
    assert_eq!(style.background_color(), CssColor::rgb(0xFF, 0xFF, 0x00));
    assert_eq!(style.color(), CssColor::BLACK);
}

#[test]
fn an_author_rule_still_beats_the_ua_phrasing_rules() {
    let style = style_of("<style>em { font-style: normal }</style><em id=target>x</em>");
    assert_eq!(style.text_advance().font_style(), FontStyle::Normal);
}

#[test]
fn a_hidden_element_generates_no_box() {
    let style = style_of("<div id=target hidden>xx</div>");
    assert_eq!(style.display(), Display::None);
}
