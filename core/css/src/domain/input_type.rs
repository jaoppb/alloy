//! [`InputType`] — the state an `<input>` element's `type` attribute selects
//! (WHATWG HTML §4.10.5).
//!
//! The attribute is an enumerated attribute with a closed vocabulary: its
//! keywords match ASCII case-insensitively, and both the *missing value
//! default* and the *invalid value default* are the Text state. Modelling it as
//! an enum keeps the one place that reads it — the label a button-like control
//! shows — free of string comparisons.

/// One state of an `<input>` element's `type` attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputType {
    Hidden,
    /// The missing- and invalid-value default.
    #[default]
    Text,
    Search,
    Telephone,
    Url,
    Email,
    Password,
    Date,
    Month,
    Week,
    Time,
    LocalDateAndTime,
    Number,
    Range,
    Color,
    Checkbox,
    RadioButton,
    FileUpload,
    SubmitButton,
    ImageButton,
    ResetButton,
    Button,
}

/// Every state, in the order WHATWG HTML §4.10.5's table lists them.
const ALL: [InputType; 22] = [
    InputType::Hidden,
    InputType::Text,
    InputType::Search,
    InputType::Telephone,
    InputType::Url,
    InputType::Email,
    InputType::Password,
    InputType::Date,
    InputType::Month,
    InputType::Week,
    InputType::Time,
    InputType::LocalDateAndTime,
    InputType::Number,
    InputType::Range,
    InputType::Color,
    InputType::Checkbox,
    InputType::RadioButton,
    InputType::FileUpload,
    InputType::SubmitButton,
    InputType::ImageButton,
    InputType::ResetButton,
    InputType::Button,
];

impl InputType {
    /// The state a `type` attribute value selects: its keyword matched ASCII
    /// case-insensitively, and the Text state for a missing or unknown value
    /// (WHATWG HTML §4.10.5).
    #[must_use]
    pub fn from_attribute(value: Option<&str>) -> Self {
        let Some(text) = value else {
            return Self::Text;
        };
        ALL.into_iter()
            .find(|state| state.keyword().eq_ignore_ascii_case(text))
            .unwrap_or(Self::Text)
    }

    /// The canonical lowercase keyword of this state.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Text => "text",
            Self::Search => "search",
            Self::Telephone => "tel",
            Self::Url => "url",
            Self::Email => "email",
            Self::Password => "password",
            Self::Date => "date",
            Self::Month => "month",
            Self::Week => "week",
            Self::Time => "time",
            Self::LocalDateAndTime => "datetime-local",
            Self::Number => "number",
            Self::Range => "range",
            Self::Color => "color",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio",
            Self::FileUpload => "file",
            Self::SubmitButton => "submit",
            Self::ImageButton => "image",
            Self::ResetButton => "reset",
            Self::Button => "button",
        }
    }

    /// The label a button-like control shows when its `value` attribute is
    /// absent (WHATWG HTML §4.10.5.1.18–20): an implementation-defined "Submit
    /// Query" / "Reset" for the submit and reset buttons, nothing for a plain
    /// button. Every other state paints no label text, so it answers `None`.
    #[must_use]
    pub const fn default_label(self) -> Option<&'static str> {
        match self {
            Self::SubmitButton => Some("Submit Query"),
            Self::ResetButton => Some("Reset"),
            Self::Button => Some(""),
            _ => None,
        }
    }

    /// The label the control actually shows: its `value` attribute when present
    /// (author text wins), else [`InputType::default_label`]. `None` for a
    /// state that shows no label at all.
    #[must_use]
    pub fn label(self, value: Option<&str>) -> Option<&str> {
        let default = self.default_label()?;
        Some(value.unwrap_or(default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keyword_round_trips_case_insensitively() {
        for state in ALL {
            let shouted = state.keyword().to_ascii_uppercase();
            assert_eq!(InputType::from_attribute(Some(&shouted)), state);
        }
    }
}
