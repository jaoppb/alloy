//! CSS Custom Properties and `var()` cascade resolution (`PRD-007`).
//!
//! Implements custom property parsing (`--*`), `var()` substitution with cycle detection,
//! fallback resolution, memoization and an expansion cap, inheritance, and
//! [`cascade_custom_properties`] — the per-element step the cascade
//! (`author_rules.rs`) runs before any ordinary declaration is applied.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::domain::computed::variables::{
    CustomPropertiesMap, VariableError, VariableName, VariableValue,
};
use crate::domain::declaration::{Declaration, DeclarationBlock, DeclarationValue};
use crate::infrastructure::cascade::variable_cycles::CyclicVariables;

/// The most text one `var()` substitution may produce: 64 KiB.
///
/// Far beyond any real stylesheet value, yet small enough that a hostile chain of doubling
/// references (`--a: var(--b) var(--b)`, thirty deep) fails in microseconds
/// instead of allocating `2^30` copies. CSS Variables L1 §3 lets a user agent
/// treat such a value as invalid at computed-value time.
pub const MAX_SUBSTITUTED_BYTES: usize = 65_536;

/// The text that opens a `var()` reference in a declaration value, rebuilt
/// from its tokens (`Token::Function("var")` prints as `var(`).
const VAR_FUNCTION_OPENING: &str = "var(";

/// Parses a single custom property name and value pair.
///
/// # Errors
///
/// Returns [`VariableError::InvalidName`] if `property_name` is not a valid custom property name.
pub fn parse_custom_property(
    property_name: &str,
    raw_value: &str,
) -> Result<(VariableName, VariableValue), VariableError> {
    let variable_name = VariableName::parse(property_name)?;
    let variable_value = VariableValue::new(raw_value);
    Ok((variable_name, variable_value))
}

/// Parses a CSS declaration block string into a [`CustomPropertiesMap`].
///
/// Only declarations whose property names start with `--` are parsed into the map;
/// standard CSS properties are skipped.
///
/// # Errors
///
/// Returns [`VariableError::InvalidName`] if a custom property name fails validation.
pub fn parse_custom_properties_block(
    css_declarations: &str,
) -> Result<CustomPropertiesMap, VariableError> {
    let mut map = CustomPropertiesMap::new();
    let mut scanner = DeclarationScanner::new(css_declarations);
    while let Some(declaration_text) = scanner.next_declaration() {
        parse_single_declaration_entry(declaration_text, &mut map)?;
    }
    Ok(map)
}

fn parse_single_declaration_entry(
    declaration_text: &str,
    map: &mut CustomPropertiesMap,
) -> Result<(), VariableError> {
    let trimmed = declaration_text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let Some(colon_position) = trimmed.find(':') else {
        return Ok(());
    };
    let name_part = trimmed.get(..colon_position).map_or("", str::trim);
    let value_part = trimmed
        .get(colon_position.saturating_add(1)..)
        .map_or("", str::trim);
    if !name_part.starts_with("--") {
        return Ok(());
    }
    let (name, value) = parse_custom_property(name_part, value_part)?;
    map.set(name, value);
    Ok(())
}

/// Extracts all custom properties from an existing [`DeclarationBlock`].
#[must_use]
pub fn extract_custom_properties(block: &DeclarationBlock) -> CustomPropertiesMap {
    let mut map = CustomPropertiesMap::new();
    for declaration in block.iter() {
        process_block_declaration(declaration, &mut map);
    }
    map
}

fn process_block_declaration(declaration: &Declaration, map: &mut CustomPropertiesMap) {
    let property_name = declaration.property().as_str();
    let Ok(variable_name) = VariableName::parse(property_name) else {
        return;
    };
    let value = VariableValue::new(declaration.value().as_str());
    map.set(variable_name, value);
}

/// Inherits custom properties from parent to child, returning a merged map.
#[must_use]
pub fn inherit_custom_properties(
    parent: &CustomPropertiesMap,
    child: &CustomPropertiesMap,
) -> CustomPropertiesMap {
    CustomPropertiesMap::inherited(parent, child)
}

/// Resolves a single variable name in a [`CustomPropertiesMap`], detecting reference cycles.
///
/// # Errors
///
/// Returns [`VariableError::UndefinedVariable`] if `name` is not defined,
/// [`VariableError::CycleDetected`] if resolving `name` enters a reference cycle, or
/// [`VariableError::ExpansionLimit`] if the substituted text outgrows
/// [`MAX_SUBSTITUTED_BYTES`].
pub fn resolve_variable(
    name: &VariableName,
    map: &CustomPropertiesMap,
) -> Result<String, VariableError> {
    Substitution::new(map, [name]).resolve_variable(name)
}

/// Checks whether a variable participates in a reference cycle.
#[must_use]
pub fn detect_cycle(name: &VariableName, map: &CustomPropertiesMap) -> Option<Vec<String>> {
    match resolve_variable(name, map) {
        Err(VariableError::CycleDetected(cycle)) => Some(cycle),
        _ => None,
    }
}

/// Substitutes all `var(...)` occurrences in `declaration_value` using `map`.
///
/// # Errors
///
/// Returns [`VariableError::UndefinedVariable`] if an unresolvable variable has no fallback,
/// [`VariableError::CycleDetected`] if a cyclic reference is unhandled,
/// [`VariableError::MalformedVarFunction`] if a `var()` function call has invalid syntax, or
/// [`VariableError::ExpansionLimit`] if the result outgrows [`MAX_SUBSTITUTED_BYTES`].
pub fn substitute_variables(
    declaration_value: &str,
    map: &CustomPropertiesMap,
) -> Result<String, VariableError> {
    let roots: Vec<VariableName> = referenced_variables(declaration_value).collect();
    Substitution::new(map, &roots).substitute(declaration_value)
}

/// Substitutes `var(...)` references in a [`DeclarationValue`].
///
/// # Errors
///
/// Returns a [`VariableError`] if variable substitution fails.
pub fn resolve_declaration_value(
    value: &DeclarationValue,
    map: &CustomPropertiesMap,
) -> Result<DeclarationValue, VariableError> {
    let substituted = substitute_variables(value.as_str(), map)?;
    Ok(DeclarationValue::new(&substituted))
}

/// Whether `value` carries a `var()` reference and so can only be read once
/// the element's custom properties are known (CSS Variables L1 §3).
#[must_use]
pub(crate) fn references_variables(value: &DeclarationValue) -> bool {
    value.as_str().contains(VAR_FUNCTION_OPENING)
}

/// A CSS-wide keyword as a custom property's whole value (CSS Cascade L4
/// §7.3). A custom property inherits, so `unset` reads exactly as `inherit`.
#[derive(Clone, Copy)]
enum CustomPropertyKeyword {
    /// The guaranteed-invalid value (CSS Variables L1 §2.2): the property is
    /// treated as never declared on this element.
    Initial,
    /// The parent's computed value, or the guaranteed-invalid value when the
    /// parent has none.
    Inherit,
}

impl CustomPropertyKeyword {
    fn of(value: &DeclarationValue) -> Option<Self> {
        match value.as_str().trim().to_ascii_lowercase().as_str() {
            "initial" => Some(Self::Initial),
            "inherit" | "unset" => Some(Self::Inherit),
            _ => None,
        }
    }
}

/// The computed custom properties of one element (CSS Variables L1 §2):
/// `parent`'s map — custom properties always inherit — overlaid with every
/// `--*` declaration of `declarations` in cascade order, then each locally
/// declared value with its own `var()` references substituted, so a child
/// inherits the substituted text rather than re-resolving it against its own
/// variables.
///
/// A locally declared value whose substitution fails (undefined without a
/// fallback, a cycle, the expansion cap) is invalid at computed-value time and
/// computes to the guaranteed-invalid value — it is removed from the map
/// (CSS Variables L1 §2.3, §3). An element that declares no custom property
/// shares its parent's map rather than copying it; one that does copies only
/// the map's entries, whose names and values are shared, never their text.
#[must_use]
pub(crate) fn cascade_custom_properties(
    parent: &Rc<CustomPropertiesMap>,
    declarations: &[&Declaration],
) -> Rc<CustomPropertiesMap> {
    let declared: Vec<(VariableName, &DeclarationValue)> = declarations
        .iter()
        .filter_map(|declaration| custom_property_of(declaration))
        .collect();
    if declared.is_empty() {
        return Rc::clone(parent);
    }
    let mut specified = CustomPropertiesMap::clone(parent);
    let mut local_names = BTreeSet::new();
    for (name, value) in declared {
        specify_custom_property(&mut specified, parent, &name, value);
        local_names.insert(name);
    }
    Rc::new(computed_custom_properties(specified, &local_names))
}

fn custom_property_of(declaration: &Declaration) -> Option<(VariableName, &DeclarationValue)> {
    let name = VariableName::new(declaration.property().as_str())?;
    Some((name, declaration.value()))
}

/// Folds one `--*` declaration into the element's specified map.
fn specify_custom_property(
    specified: &mut CustomPropertiesMap,
    parent: &CustomPropertiesMap,
    name: &VariableName,
    value: &DeclarationValue,
) {
    match CustomPropertyKeyword::of(value) {
        Some(CustomPropertyKeyword::Initial) => {
            specified.remove(name);
        }
        Some(CustomPropertyKeyword::Inherit) => inherit_custom_property(specified, parent, name),
        None => specified.set(name.clone(), VariableValue::new(value.as_str())),
    }
}

fn inherit_custom_property(
    specified: &mut CustomPropertiesMap,
    parent: &CustomPropertiesMap,
    name: &VariableName,
) {
    let Some(inherited) = parent.get(name) else {
        specified.remove(name);
        return;
    };
    specified.set(name.clone(), inherited.clone());
}

/// `specified` with every locally declared `var()`-bearing value replaced by
/// its substitution, or removed when that substitution fails.
fn computed_custom_properties(
    mut specified: CustomPropertiesMap,
    local_names: &BTreeSet<VariableName>,
) -> CustomPropertiesMap {
    let mut substitution = Substitution::new(&specified, local_names);
    let resolutions: Vec<(VariableName, Result<String, VariableError>)> = local_names
        .iter()
        .filter(|name| specified.get(name).is_some_and(VariableValue::contains_var))
        .map(|name| (name.clone(), substitution.resolve_variable(name)))
        .collect();
    for (name, resolution) in resolutions {
        record_resolution(&mut specified, name, resolution);
    }
    specified
}

fn record_resolution(
    computed: &mut CustomPropertiesMap,
    name: VariableName,
    resolution: Result<String, VariableError>,
) {
    match resolution {
        Ok(text) => computed.set(name, VariableValue::new(&text)),
        Err(error) => {
            tracing::debug!(variable = %name, %error, "custom property is invalid at computed-value time");
            computed.remove(&name);
        }
    }
}

/// One substitution run over one [`CustomPropertiesMap`].
///
/// `resolved` memoizes every variable this run has already resolved, so a
/// variable referenced many times is expanded once; `stack` is the chain of
/// variables currently being expanded, which is what detects a cycle as it is
/// walked; `cyclic` names every variable on a cycle up front, so one whose
/// cycle a fallback would hide is still invalid. Every append is checked
/// against [`MAX_SUBSTITUTED_BYTES`], so even a chain whose every link
/// doubles its predecessor fails after a bounded amount of work.
struct Substitution<'map> {
    map: &'map CustomPropertiesMap,
    resolved: BTreeMap<VariableName, Result<String, VariableError>>,
    stack: Vec<VariableName>,
    cyclic: CyclicVariables,
}

impl<'map> Substitution<'map> {
    /// A run over `map` that will start from `roots` — the variables whose
    /// cycles it needs to know about.
    fn new<'roots>(
        map: &'map CustomPropertiesMap,
        roots: impl IntoIterator<Item = &'roots VariableName>,
    ) -> Self {
        Self {
            map,
            resolved: BTreeMap::new(),
            stack: Vec::new(),
            cyclic: CyclicVariables::reachable_from(map, roots),
        }
    }

    fn resolve_variable(&mut self, name: &VariableName) -> Result<String, VariableError> {
        if let Some(memoized) = self.resolved.get(name) {
            return memoized.clone();
        }
        check_cycle(&self.stack, name)?;
        let Some(raw_value) = self.map.get(name) else {
            return Err(VariableError::UndefinedVariable(name.as_str().to_owned()));
        };
        self.stack.push(name.clone());
        let substituted = self.substitute(raw_value.as_str());
        self.stack.pop();
        let resolution = self.invalid_when_cyclic(name, substituted);
        self.resolved.insert(name.clone(), resolution.clone());
        resolution
    }

    /// `resolution`, unless `name` is on a cycle: then it is invalid (CSS
    /// Variables L1 §2.3) even when a fallback on the way produced text.
    fn invalid_when_cyclic(
        &self,
        name: &VariableName,
        resolution: Result<String, VariableError>,
    ) -> Result<String, VariableError> {
        if let Err(VariableError::CycleDetected(path)) = resolution {
            return Err(VariableError::CycleDetected(path));
        }
        let Some(cycle) = self.cyclic.cycle_through(name) else {
            return resolution;
        };
        Err(VariableError::CycleDetected(cycle))
    }

    fn substitute(&mut self, input: &str) -> Result<String, VariableError> {
        let mut cursor: usize = 0;
        let mut output = String::new();
        let mut scanner = VarCallScanner::new(input);
        while let Some(var_start) = scanner.find_next_from(cursor) {
            append_bounded(
                &mut output,
                input.get(cursor..var_start).unwrap_or_default(),
            )?;
            let advance_by = self.substitute_call_at(input, var_start, &mut output)?;
            cursor = var_start.saturating_add(advance_by);
        }
        append_bounded(&mut output, input.get(cursor..).unwrap_or_default())?;
        Ok(output)
    }

    /// Expands the `var(...)` starting at `var_start` into `output`, answering
    /// how many bytes of `input` it spanned.
    fn substitute_call_at(
        &mut self,
        input: &str,
        var_start: usize,
        output: &mut String,
    ) -> Result<usize, VariableError> {
        let arguments_start = var_start.saturating_add(VAR_FUNCTION_OPENING.len());
        let Some(after_var) = input.get(arguments_start..) else {
            return Err(VariableError::MalformedVarFunction(
                "truncated `var(`".to_owned(),
            ));
        };
        let Some(relative_close) = find_matching_close_paren(after_var) else {
            return Err(VariableError::MalformedVarFunction(
                "unclosed `var()`".to_owned(),
            ));
        };
        let arguments_slice = after_var.get(..relative_close).unwrap_or_default();
        let (name_part, fallback_part) = split_var_arguments(arguments_slice);
        let resolved = self.resolve_call(name_part, fallback_part)?;
        append_bounded(output, &resolved)?;
        Ok(relative_close.saturating_add(VAR_FUNCTION_OPENING.len().saturating_add(1)))
    }

    /// `var(name)` or `var(name, fallback)`: the variable's value, else the
    /// fallback substituted in turn, else the variable's own error.
    fn resolve_call(
        &mut self,
        name_text: &str,
        fallback: Option<&str>,
    ) -> Result<String, VariableError> {
        let variable_name = VariableName::parse(name_text)?;
        let resolution = self.resolve_variable(&variable_name);
        match (resolution, fallback) {
            (Ok(value), _) => Ok(value),
            (Err(error), None) => Err(error),
            (Err(_), Some(fallback_text)) => self.substitute(fallback_text),
        }
    }
}

/// Every variable `value` names in a `var()`, fallbacks included and in
/// order — the edges of the reference graph (CSS Variables L1 §2.3). A
/// malformed call contributes nothing; substitution reports it.
pub(super) fn referenced_variables(value: &str) -> impl Iterator<Item = VariableName> + '_ {
    let mut scanner = VarCallScanner::new(value);
    let mut cursor = 0;
    core::iter::from_fn(move || {
        let var_start = scanner.find_next_from(cursor)?;
        let arguments_start = var_start.saturating_add(VAR_FUNCTION_OPENING.len());
        cursor = arguments_start;
        Some(variable_named_at(value, arguments_start))
    })
    .flatten()
}

/// The variable a `var(` call whose arguments start at `arguments_start`
/// names, if the call is well formed.
fn variable_named_at(value: &str, arguments_start: usize) -> Option<VariableName> {
    let after_var = value.get(arguments_start..)?;
    let arguments = after_var.get(..find_matching_close_paren(after_var)?)?;
    let (name_part, _) = split_var_arguments(arguments);
    VariableName::new(name_part)
}

/// Appends `text` to `output` unless that would push it past
/// [`MAX_SUBSTITUTED_BYTES`].
fn append_bounded(output: &mut String, text: &str) -> Result<(), VariableError> {
    if output.len().saturating_add(text.len()) > MAX_SUBSTITUTED_BYTES {
        return Err(VariableError::ExpansionLimit {
            limit_bytes: MAX_SUBSTITUTED_BYTES,
        });
    }
    output.push_str(text);
    Ok(())
}

fn check_cycle(stack: &[VariableName], target: &VariableName) -> Result<(), VariableError> {
    if stack.contains(target) {
        let cycle = build_cycle_path(stack, target);
        return Err(VariableError::CycleDetected(cycle));
    }
    Ok(())
}

fn build_cycle_path(stack: &[VariableName], target: &VariableName) -> Vec<String> {
    let start_index = stack.iter().position(|item| item == target).unwrap_or(0);
    let slice = stack.get(start_index..).unwrap_or_default();
    let mut path: Vec<String> = slice.iter().map(|item| item.as_str().to_owned()).collect();
    path.push(target.as_str().to_owned());
    path
}

fn split_var_arguments(arguments_text: &str) -> (&str, Option<&str>) {
    let Some(split_index) = find_argument_split(arguments_text) else {
        return (arguments_text.trim(), None);
    };
    let name_part = arguments_text.get(..split_index).map_or("", str::trim);
    let fallback_part = arguments_text
        .get(split_index.saturating_add(1)..)
        .map_or("", str::trim);
    (name_part, Some(fallback_part))
}

struct VarCallScanner<'a> {
    slice: &'a str,
    in_single_quote: bool,
    in_double_quote: bool,
}

impl<'a> VarCallScanner<'a> {
    const fn new(slice: &'a str) -> Self {
        Self {
            slice,
            in_single_quote: false,
            in_double_quote: false,
        }
    }

    fn find_next_from(&mut self, start_from: usize) -> Option<usize> {
        let mut cursor = start_from;
        while cursor < self.slice.len() {
            let remaining = self.slice.get(cursor..)?;
            let character = remaining.chars().next()?;
            if self.check_match(remaining) {
                return Some(cursor);
            }
            self.advance_cursor(&mut cursor, character);
        }
        None
    }

    fn check_match(&self, remaining: &str) -> bool {
        self.outside_quotes() && remaining.starts_with("var(")
    }

    const fn advance_cursor(&mut self, cursor: &mut usize, character: char) {
        match character {
            '\'' if !self.in_double_quote => self.in_single_quote = !self.in_single_quote,
            '"' if !self.in_single_quote => self.in_double_quote = !self.in_double_quote,
            _ => {}
        }
        *cursor = cursor.saturating_add(character.len_utf8());
    }

    const fn outside_quotes(&self) -> bool {
        !self.in_single_quote && !self.in_double_quote
    }
}

struct ParenScanner<'a> {
    chars: std::str::CharIndices<'a>,
    depth: usize,
    in_single_quote: bool,
    in_double_quote: bool,
}

impl<'a> ParenScanner<'a> {
    fn new(slice: &'a str) -> Self {
        Self {
            chars: slice.char_indices(),
            depth: 1,
            in_single_quote: false,
            in_double_quote: false,
        }
    }

    const fn step(&mut self, (index, character): (usize, char)) -> Option<usize> {
        let is_closing = self.process_char(character);
        if is_closing {
            return Some(index);
        }
        None
    }

    const fn process_char(&mut self, character: char) -> bool {
        match character {
            '\'' if !self.in_double_quote => self.toggle_single_quote(),
            '"' if !self.in_single_quote => self.toggle_double_quote(),
            '(' if self.outside_quotes() => self.depth = self.depth.saturating_add(1),
            ')' if self.outside_quotes() => {
                self.depth = self.depth.saturating_sub(1);
                return self.depth == 0;
            }
            _ => {}
        }
        false
    }

    const fn outside_quotes(&self) -> bool {
        !self.in_single_quote && !self.in_double_quote
    }

    const fn toggle_single_quote(&mut self) {
        self.in_single_quote = !self.in_single_quote;
    }

    const fn toggle_double_quote(&mut self) {
        self.in_double_quote = !self.in_double_quote;
    }
}

fn find_matching_close_paren(slice: &str) -> Option<usize> {
    let mut scanner = ParenScanner::new(slice);
    while let Some(item) = scanner.chars.next() {
        if let Some(index) = scanner.step(item) {
            return Some(index);
        }
    }
    None
}

struct ArgumentScanner<'a> {
    chars: std::str::CharIndices<'a>,
    depth: usize,
    in_single_quote: bool,
    in_double_quote: bool,
}

impl<'a> ArgumentScanner<'a> {
    fn new(slice: &'a str) -> Self {
        Self {
            chars: slice.char_indices(),
            depth: 0,
            in_single_quote: false,
            in_double_quote: false,
        }
    }

    const fn step(&mut self, (index, character): (usize, char)) -> Option<usize> {
        match character {
            '\'' if !self.in_double_quote => self.in_single_quote = !self.in_single_quote,
            '"' if !self.in_single_quote => self.in_double_quote = !self.in_double_quote,
            '(' if self.outside_quotes() => self.depth = self.depth.saturating_add(1),
            ')' if self.outside_quotes() => self.depth = self.depth.saturating_sub(1),
            ',' if self.depth == 0 && self.outside_quotes() => return Some(index),
            _ => {}
        }
        None
    }

    const fn outside_quotes(&self) -> bool {
        !self.in_single_quote && !self.in_double_quote
    }
}

fn find_argument_split(slice: &str) -> Option<usize> {
    let mut scanner = ArgumentScanner::new(slice);
    while let Some(item) = scanner.chars.next() {
        if let Some(index) = scanner.step(item) {
            return Some(index);
        }
    }
    None
}

struct DeclarationScanner<'a> {
    slice: &'a str,
    cursor: usize,
    start: usize,
    depth: usize,
    in_single_quote: bool,
    in_double_quote: bool,
}

impl<'a> DeclarationScanner<'a> {
    const fn new(slice: &'a str) -> Self {
        Self {
            slice,
            cursor: 0,
            start: 0,
            depth: 0,
            in_single_quote: false,
            in_double_quote: false,
        }
    }

    fn next_declaration(&mut self) -> Option<&'a str> {
        while self.cursor < self.slice.len() {
            let remaining = self.slice.get(self.cursor..)?;
            let character = remaining.chars().next()?;
            let character_length = character.len_utf8();
            if self.is_delimiter(character) {
                let declaration = self.slice.get(self.start..self.cursor);
                self.cursor = self.cursor.saturating_add(character_length);
                self.start = self.cursor;
                return declaration;
            }
            self.advance_state(character, character_length);
        }
        self.finish_remaining()
    }

    const fn is_delimiter(&self, character: char) -> bool {
        character == ';' && self.depth == 0 && self.outside_quotes()
    }

    const fn advance_state(&mut self, character: char, character_length: usize) {
        match character {
            '\'' if !self.in_double_quote => self.in_single_quote = !self.in_single_quote,
            '"' if !self.in_single_quote => self.in_double_quote = !self.in_double_quote,
            '(' if self.outside_quotes() => self.depth = self.depth.saturating_add(1),
            ')' if self.outside_quotes() => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        self.cursor = self.cursor.saturating_add(character_length);
    }

    fn finish_remaining(&mut self) -> Option<&'a str> {
        if self.start < self.slice.len() {
            let remaining = self.slice.get(self.start..);
            self.start = self.slice.len();
            return remaining;
        }
        None
    }

    const fn outside_quotes(&self) -> bool {
        !self.in_single_quote && !self.in_double_quote
    }
}
