//! [`HtmlEntity`] — strongly-typed HTML named character entities per W3C specification.

use core::fmt;

macro_rules! define_entities {
    ( $( $variant:ident => ( $ch:literal, $name:literal ) ),* $(,)? ) => {
        /// A strongly-typed W3C HTML named character entity.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum HtmlEntity {
            $(
                #[doc = concat!("The `&", $name, ";` entity (`", $ch, "`).")]
                $variant,
            )*
        }

        impl HtmlEntity {
            /// Lookup a named entity by its unicode character.
            #[must_use]
            pub const fn from_char(ch: char) -> Option<Self> {
                match ch {
                    $( $ch => Some(Self::$variant), )*
                    _ => None,
                }
            }

            /// Lookup a named entity by its entity name without delimiters (e.g. `"copy"` for `&copy;`).
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $( $name => Some(Self::$variant), )*
                    _ => None,
                }
            }

            /// The single character corresponding to this entity.
            #[must_use]
            pub const fn as_char(&self) -> char {
                match self {
                    $( Self::$variant => $ch, )*
                }
            }

            /// The entity name without delimiters (e.g. `"copy"` for `&copy;`).
            #[must_use]
            pub const fn entity_name(&self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )*
                }
            }

            /// The full serialized entity string including delimiters (e.g. `"&copy;"`).
            #[must_use]
            pub const fn as_entity(&self) -> &'static str {
                match self {
                    $( Self::$variant => concat!("&", $name, ";"), )*
                }
            }
        }

        impl fmt::Display for HtmlEntity {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_entity())
            }
        }
    };
}

define_entities! {
    // Markup & Delimiters
    Amp => ('&', "amp"),
    Lt => ('<', "lt"),
    Gt => ('>', "gt"),
    Quot => ('"', "quot"),
    Apos => ('\'', "apos"),

    // Whitespace
    Nbsp => ('\u{00A0}', "nbsp"),

    // Currencies
    Cent => ('\u{00A2}', "cent"),
    Pound => ('\u{00A3}', "pound"),
    Yen => ('\u{00A5}', "yen"),
    Euro => ('\u{20AC}', "euro"),

    // Legal / Marks
    Sect => ('\u{00A7}', "sect"),
    Copy => ('\u{00A9}', "copy"),
    Laquo => ('\u{00AB}', "laquo"),
    Reg => ('\u{00AE}', "reg"),
    Deg => ('\u{00B0}', "deg"),
    Plusmn => ('\u{00B1}', "plusmn"),
    Micro => ('\u{00B5}', "micro"),
    Middot => ('\u{00B7}', "middot"),
    Raquo => ('\u{00BB}', "raquo"),
    Frac14 => ('\u{00BC}', "frac14"),
    Frac12 => ('\u{00BD}', "frac12"),
    Frac34 => ('\u{00BE}', "frac34"),
    Times => ('\u{00D7}', "times"),
    Divide => ('\u{00F7}', "divide"),
    Trade => ('\u{2122}', "trade"),

    // Punctuation & Typography
    Ndash => ('\u{2013}', "ndash"),
    Mdash => ('\u{2014}', "mdash"),
    Lsquo => ('\u{2018}', "lsquo"),
    Rsquo => ('\u{2019}', "rsquo"),
    Ldquo => ('\u{201C}', "ldquo"),
    Rdquo => ('\u{201D}', "rdquo"),
    Bull => ('\u{2022}', "bull"),
    Hellip => ('\u{2026}', "hellip"),

    // Arrows & Math
    Larr => ('\u{2190}', "larr"),
    Uarr => ('\u{2191}', "uarr"),
    Rarr => ('\u{2192}', "rarr"),
    Darr => ('\u{2193}', "darr"),
    Harr => ('\u{2194}', "harr"),
    Asymp => ('\u{2248}', "asymp"),
    Ne => ('\u{2260}', "ne"),
    Le => ('\u{2264}', "le"),
    Ge => ('\u{2265}', "ge"),
}
