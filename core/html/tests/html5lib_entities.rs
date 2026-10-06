//! Replays html5lib-tests' character-reference cases (`tests/data/html5lib/`) against the tokenizer
//! and asserts the emitted tokens, the parse-error codes and their locations. No case is skipped (#31).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use html::{HtmlError, SourceLocation, Token, TokenSink, TokenSinkResult, Tokenizer};
use serde_json::Value;

/// Collects everything the tokenizer emits.
#[derive(Default)]
struct Recorder {
    tokens: Vec<Token>,
}

impl TokenSink for Recorder {
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError> {
        self.tokens.push(token);
        Ok(TokenSinkResult::Continue)
    }

    fn finish(&mut self) -> Result<(), HtmlError> {
        Ok(())
    }
}

/// A token in html5lib's JSON spelling, adjacent characters merged: `["Character", "a"]`,
/// `["StartTag", "h", {"a": "b"}]`.
type Emitted = Vec<Value>;

/// `(code, line, column)` as html5lib reports it.
type Diagnostic = (String, usize, usize);

/// `value[key]` without indexing; a fixture missing a field is a malformed fixture.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("fixture entry has no `{key}`: {value}"))
}

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/html5lib")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn tokenize(input: &str) -> (Emitted, Vec<(String, SourceLocation)>) {
    let mut recorder = Recorder::default();
    Tokenizer::new(input).run(&mut recorder).unwrap();
    let mut emitted: Emitted = Vec::new();
    let mut characters = String::new();
    let mut diagnostics = Vec::new();
    for token in recorder.tokens {
        match token {
            Token::Character(text) => characters.push_str(text.as_str()),
            Token::StartTag(tag) => {
                flush_characters(&mut emitted, &mut characters);
                emitted.push(start_tag(&tag));
            }
            Token::ParseError(diagnostic) => {
                diagnostics.push((
                    diagnostic.code().as_str().to_string(),
                    diagnostic.location(),
                ));
            }
            _ => {}
        }
    }
    flush_characters(&mut emitted, &mut characters);
    (emitted, diagnostics)
}

/// html5lib reports one `Character` token for a run of text; the tokenizer may split it.
fn flush_characters(emitted: &mut Emitted, characters: &mut String) {
    if characters.is_empty() {
        return;
    }
    emitted.push(serde_json::json!(["Character", std::mem::take(characters)]));
}

fn start_tag(tag: &html::TagToken) -> Value {
    let attributes: BTreeMap<&str, &str> = tag
        .attributes()
        .iter()
        .map(|entry| (entry.name().as_str(), entry.value().as_str()))
        .collect();
    serde_json::json!(["StartTag", tag.name(), attributes])
}

/// Codes html5lib locates where the reference was *detected* (after `&name`, at the `;`, after the
/// `;`); alloy locates every one of them at the `&` that opens the reference (ADR-0023, #36).
const REFERENCE_CODES: [&str; 6] = [
    "missing-semicolon-after-character-reference",
    "unknown-named-character-reference",
    "control-character-reference",
    "noncharacter-character-reference",
    "null-character-reference",
    "absence-of-digits-in-numeric-character-reference",
];

/// The one documented rule translating html5lib's location into alloy's.
///
/// A character-reference error maps to the `&` closest before html5lib's position — references never
/// nest, so that `&` opens the reference being reported. Any other error is located identically.
fn to_alloy_location(input: &str, (code, line, column): &Diagnostic) -> (usize, usize) {
    if !REFERENCE_CODES.contains(&code.as_str()) {
        return (*line, *column);
    }
    let detected = characters_before(input, *line, *column);
    let ampersand = input
        .chars()
        .take(detected)
        .collect::<Vec<_>>()
        .iter()
        .rposition(|character| *character == '&')
        .unwrap_or_else(|| panic!("no `&` before {line}:{column} in {input:?}"));
    line_and_column(input, ampersand)
}

/// How many characters precede the 1-based `(line, column)`.
fn characters_before(input: &str, line: usize, column: usize) -> usize {
    let preceding_lines: usize = input
        .split('\n')
        .take(line.saturating_sub(1))
        .map(|text| text.chars().count().saturating_add(1))
        .sum();
    preceding_lines.saturating_add(column).saturating_sub(1)
}

/// The 1-based `(line, column)` of the character at `index`.
fn line_and_column(input: &str, index: usize) -> (usize, usize) {
    let preceding: String = input.chars().take(index).collect();
    let line = preceding.matches('\n').count().saturating_add(1);
    let column = preceding
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        .saturating_add(1);
    (line, column)
}

fn expected_diagnostics(case: &Value) -> Vec<Diagnostic> {
    let Some(errors) = case.get("errors").and_then(Value::as_array) else {
        return Vec::new();
    };
    errors
        .iter()
        .map(|error| {
            (
                field(error, "code").as_str().unwrap().to_string(),
                usize::try_from(field(error, "line").as_u64().unwrap()).unwrap(),
                usize::try_from(field(error, "col").as_u64().unwrap()).unwrap(),
            )
        })
        .collect()
}

/// Returns a description of what differs, or `None` when the case passes.
fn mismatch(case: &Value) -> Option<String> {
    let input = field(case, "input").as_str().unwrap();
    let (emitted, found) = tokenize(input);
    let expected_tokens: Emitted = field(case, "output").as_array().unwrap().clone();
    if emitted != expected_tokens {
        return Some(format!("tokens {emitted:?}, expected {expected_tokens:?}"));
    }
    let expected: Vec<(String, (usize, usize))> = expected_diagnostics(case)
        .iter()
        .map(|diagnostic| (diagnostic.0.clone(), to_alloy_location(input, diagnostic)))
        .collect();
    let found: Vec<(String, (usize, usize))> = found
        .into_iter()
        .map(|(code, location)| (code, (location.line(), location.column())))
        .collect();
    (found != expected).then(|| format!("diagnostics {found:?}, expected {expected:?}"))
}

fn replay(name: &str) -> usize {
    let fixture = fixture(name);
    let cases = fixture.get("tests").and_then(Value::as_array).unwrap();
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| {
            mismatch(case).map(|difference| format!("{:?}: {difference}", field(case, "input")))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases in {name} fail; first ones:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    cases.len()
}

#[test]
fn every_named_reference_resolves_as_html5lib_expects() {
    assert_eq!(replay("namedEntities.test"), 4210);
}

#[test]
fn the_attribute_and_numeric_reference_cases_match_html5lib() {
    assert_eq!(replay("entities.test"), 80);
}
