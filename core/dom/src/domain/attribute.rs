use std::collections::HashMap;
use std::fmt;

/// Strongly typed HTML/XML attribute name mapped to standard W3C attributes with open families for `data-*`, `aria-*`, and custom attributes (C-22).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeName {
    // Global attributes
    Id,
    Class,
    Style,
    Title,
    Lang,
    Dir,
    Tabindex,
    Hidden,

    // Links & Resources
    Href,
    Src,
    Alt,
    Width,
    Height,

    // Forms
    Type,
    Value,
    Name,
    Placeholder,
    Checked,
    Disabled,
    Selected,
    Readonly,
    Required,
    Action,
    Method,

    // Metadata & Links
    Rel,
    Target,

    // Extensible open families (C-22)
    Data(String),
    Aria(String),
    Custom(String),
}

impl AttributeName {
    /// Creates and normalizes a new `AttributeName`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let raw = name.into();
        let trimmed = raw.trim();
        let lower = trimmed.to_ascii_lowercase();

        match lower.as_str() {
            "id" => Self::Id,
            "class" => Self::Class,
            "style" => Self::Style,
            "title" => Self::Title,
            "lang" => Self::Lang,
            "dir" => Self::Dir,
            "tabindex" => Self::Tabindex,
            "hidden" => Self::Hidden,
            "href" => Self::Href,
            "src" => Self::Src,
            "alt" => Self::Alt,
            "width" => Self::Width,
            "height" => Self::Height,
            "type" => Self::Type,
            "value" => Self::Value,
            "name" => Self::Name,
            "placeholder" => Self::Placeholder,
            "checked" => Self::Checked,
            "disabled" => Self::Disabled,
            "selected" => Self::Selected,
            "readonly" => Self::Readonly,
            "required" => Self::Required,
            "action" => Self::Action,
            "method" => Self::Method,
            "rel" => Self::Rel,
            "target" => Self::Target,
            _ if lower.starts_with("data-") => Self::Data(lower),
            _ if lower.starts_with("aria-") => Self::Aria(lower),
            _ => Self::Custom(lower),
        }
    }

    /// Accesses the attribute name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Id => "id",
            Self::Class => "class",
            Self::Style => "style",
            Self::Title => "title",
            Self::Lang => "lang",
            Self::Dir => "dir",
            Self::Tabindex => "tabindex",
            Self::Hidden => "hidden",
            Self::Href => "href",
            Self::Src => "src",
            Self::Alt => "alt",
            Self::Width => "width",
            Self::Height => "height",
            Self::Type => "type",
            Self::Value => "value",
            Self::Name => "name",
            Self::Placeholder => "placeholder",
            Self::Checked => "checked",
            Self::Disabled => "disabled",
            Self::Selected => "selected",
            Self::Readonly => "readonly",
            Self::Required => "required",
            Self::Action => "action",
            Self::Method => "method",
            Self::Rel => "rel",
            Self::Target => "target",
            Self::Data(s) | Self::Aria(s) | Self::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Strongly typed attribute value (e.g. `container`, `https://example.com`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeValue(String);

impl AttributeValue {
    /// Creates a new `AttributeValue`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Accesses the value as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// First-class collection wrapping element attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeMap {
    entries: HashMap<AttributeName, AttributeValue>,
}

impl AttributeMap {
    /// Creates an empty attribute map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Inserts an attribute into the map.
    pub fn insert(&mut self, name: AttributeName, value: AttributeValue) {
        self.entries.insert(name, value);
    }

    /// Gets an attribute value by name.
    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&AttributeValue> {
        self.entries.get(name)
    }

    /// Checks whether an attribute exists in the map.
    #[must_use]
    pub fn contains(&self, name: &AttributeName) -> bool {
        self.entries.contains_key(name)
    }

    /// Removes an attribute from the map.
    pub fn remove(&mut self, name: &AttributeName) -> Option<AttributeValue> {
        self.entries.remove(name)
    }

    /// Returns the number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks whether the attribute map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over attribute key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&AttributeName, &AttributeValue)> {
        self.entries.iter()
    }
}
