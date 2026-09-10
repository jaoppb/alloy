//! Tokenizer state machine state enum.

use crate::application::ports::RawKind;

/// Internal tokenizer state machine states compliant with WHATWG §13.2.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    EndTagName,
    AfterEndTagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    MarkupDeclarationOpen,
    Comment,
    BogusComment,
    Doctype,
    RawText(RawKind),
}
