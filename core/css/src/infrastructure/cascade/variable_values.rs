//! CSS Custom Properties and `var()` cascade resolution (`PRD-007`).
//!
//! Implements custom property parsing (`--*`), `var()` substitution with cycle detection,
//! fallback resolution, and inheritance.

use crate::domain::computed::variables::{
    CustomPropertiesMap, VariableError, VariableName, VariableValue,
};
use crate::domain::declaration::{Declaration, DeclarationBlock, DeclarationValue};

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
/// Returns [`VariableError::UndefinedVariable`] if `name` is not defined, or
/// [`VariableError::CycleDetected`] if resolving `name` enters a reference cycle.
pub fn resolve_variable(
    name: &VariableName,
    map: &CustomPropertiesMap,
) -> Result<String, VariableError> {
    let mut stack = Vec::new();
    resolve_variable_internal(name, map, &mut stack)
}

/// Checks whether a variable participates in a reference cycle.
#[must_use]
pub fn detect_cycle(name: &VariableName, map: &CustomPropertiesMap) -> Option<Vec<String>> {
    let mut stack = Vec::new();
    match resolve_variable_internal(name, map, &mut stack) {
        Err(VariableError::CycleDetected(cycle)) => Some(cycle),
        _ => None,
    }
}

/// Substitutes all `var(...)` occurrences in `declaration_value` using `map`.
///
/// # Errors
///
/// Returns [`VariableError::UndefinedVariable`] if an unresolvable variable has no fallback,
/// [`VariableError::CycleDetected`] if a cyclic reference is unhandled, or
/// [`VariableError::MalformedVarFunction`] if a `var()` function call has invalid syntax.
pub fn substitute_variables(
    declaration_value: &str,
    map: &CustomPropertiesMap,
) -> Result<String, VariableError> {
    let mut stack = Vec::new();
    substitute_variables_internal(declaration_value, map, &mut stack)
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

fn resolve_variable_internal(
    name: &VariableName,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
) -> Result<String, VariableError> {
    check_cycle(stack, name)?;
    let Some(raw_value) = map.get(name) else {
        return Err(VariableError::UndefinedVariable(name.as_str().to_owned()));
    };
    stack.push(name.clone());
    let resolution_result = substitute_variables_internal(raw_value.as_str(), map, stack);
    stack.pop();
    resolution_result
}

fn check_cycle(stack: &[VariableName], target: &VariableName) -> Result<(), VariableError> {
    if stack.contains(target) {
        let cycle = build_cycle_path(stack, target);
        return Err(VariableError::CycleDetected(cycle));
    }
    Ok(())
}

fn build_cycle_path(stack: &[VariableName], target: &VariableName) -> Vec<String> {
    let position = stack.iter().position(|item| item == target);
    let start_index = position.map_or(0, |index| index);
    let slice = stack.get(start_index..).map_or(&[][..], |items| items);
    let mut path: Vec<String> = slice.iter().map(|item| item.as_str().to_owned()).collect();
    path.push(target.as_str().to_owned());
    path
}

fn substitute_variables_internal(
    input: &str,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
) -> Result<String, VariableError> {
    let mut cursor: usize = 0;
    let mut output = String::new();
    let mut scanner = VarCallScanner::new(input);
    while let Some(var_start) = scanner.find_next_from(cursor) {
        append_prefix(input, cursor, var_start, &mut output);
        let advance_by = process_var_at(input, var_start, map, stack, &mut output)?;
        cursor = var_start.saturating_add(advance_by);
    }
    append_remaining(input, cursor, &mut output);
    Ok(output)
}

fn append_prefix(input: &str, cursor: usize, var_start: usize, output: &mut String) {
    if let Some(prefix) = input.get(cursor..var_start) {
        output.push_str(prefix);
    }
}

fn append_remaining(input: &str, cursor: usize, output: &mut String) {
    if let Some(remaining) = input.get(cursor..) {
        output.push_str(remaining);
    }
}

fn process_var_at(
    input: &str,
    var_start: usize,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
    output: &mut String,
) -> Result<usize, VariableError> {
    let arguments_start = var_start.saturating_add(4);
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
    let arguments_slice = after_var.get(..relative_close).map_or("", |text| text);
    let (name_part, fallback_part) = split_var_arguments(arguments_slice);
    let resolved = resolve_var_call(name_part, fallback_part, map, stack)?;
    output.push_str(&resolved);
    Ok(relative_close.saturating_add(5))
}

fn resolve_var_call(
    name_str: &str,
    fallback: Option<&str>,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
) -> Result<String, VariableError> {
    let variable_name = VariableName::parse(name_str)?;
    let result = resolve_variable_internal(&variable_name, map, stack);
    handle_resolution_fallback(result, fallback, map, stack)
}

fn handle_resolution_fallback(
    result: Result<String, VariableError>,
    fallback: Option<&str>,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
) -> Result<String, VariableError> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => resolve_fallback_or_error(err, fallback, map, stack),
    }
}

fn resolve_fallback_or_error(
    error: VariableError,
    fallback: Option<&str>,
    map: &CustomPropertiesMap,
    stack: &mut Vec<VariableName>,
) -> Result<String, VariableError> {
    let Some(fallback_text) = fallback else {
        return Err(error);
    };
    substitute_variables_internal(fallback_text, map, stack)
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
            let char_len = character.len_utf8();
            if self.is_delimiter(character) {
                let decl = self.slice.get(self.start..self.cursor);
                self.cursor = self.cursor.saturating_add(char_len);
                self.start = self.cursor;
                return decl;
            }
            self.advance_state(character, char_len);
        }
        self.finish_remaining()
    }

    const fn is_delimiter(&self, character: char) -> bool {
        character == ';' && self.depth == 0 && self.outside_quotes()
    }

    const fn advance_state(&mut self, character: char, len: usize) {
        match character {
            '\'' if !self.in_double_quote => self.in_single_quote = !self.in_single_quote,
            '"' if !self.in_single_quote => self.in_double_quote = !self.in_double_quote,
            '(' if self.outside_quotes() => self.depth = self.depth.saturating_add(1),
            ')' if self.outside_quotes() => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        self.cursor = self.cursor.saturating_add(len);
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
