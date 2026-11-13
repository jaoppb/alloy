//! The values the embedded UA sheet gives HTML elements (WHATWG HTML §15.3,
//! issue #133 of #81): the computed style a page gets with no author CSS at
//! all, through the whole chain `dom::parse → snapshot → collect_style_sheets
//! → UaCascade::resolve`. Painting the phrasing values is #57's; this file
//! pins the cascade. The `display` of every element is pinned by the unit
//! tests of `src/infrastructure/ua_sheet.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::domain::computed::inline_style::{TextAlign, WhiteSpace};
use css::domain::computed::text_advance::{FontStyle, FontWeight, TextDecoration};
use css::{
    CascadeResolver, ComputedStyle, CssColor, FontFamily, GenericFamily, Length, LengthEdges,
    UaCascade, collect_style_sheets, snapshot,
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
fn nobr_does_not_wrap() {
    let style = style_of("<nobr id=target>x</nobr>");
    assert_eq!(style.white_space(), WhiteSpace::NoWrap);
}

#[test]
fn a_list_nested_in_a_list_has_no_block_margins() {
    let outer = style_of("<ul id=target><li>x</li></ul>");
    assert_eq!(
        outer.margin().top(),
        Length::Em(1.0),
        "a top-level list keeps its 1em"
    );
    for (outer_tag, inner_tag) in [("ul", "ol"), ("ol", "ul"), ("dl", "menu"), ("ul", "dl")] {
        let nested = style_of(&format!(
            "<{outer_tag}><li><{inner_tag} id=target></{inner_tag}></li></{outer_tag}>"
        ));
        assert_eq!(
            nested.margin().top(),
            Length::Pixels(0.0),
            "{outer_tag} {inner_tag}"
        );
        assert_eq!(
            nested.margin().bottom(),
            Length::Pixels(0.0),
            "{outer_tag} {inner_tag}"
        );
    }
}

#[test]
fn table_cells_and_captions_carry_the_spec_values() {
    let table = |cell: &str| style_of(&format!("<table>{cell}</table>"));
    let caption = table("<caption id=target>x</caption>");
    assert_eq!(caption.text_align(), TextAlign::Center);

    let header = table("<tr><th id=target>x</th></tr>");
    assert_eq!(header.text_advance().font_weight(), FontWeight::BOLD);
    assert_eq!(header.padding(), LengthEdges::uniform(Length::Pixels(1.0)));

    let data = table("<tr><td id=target>x</td></tr>");
    assert_eq!(data.padding(), LengthEdges::uniform(Length::Pixels(1.0)));
}
