//! Turning fetched stylesheet text into something the cascade can absorb.

use css::{MediaQuery, Origin, StyleSheetSet};

/// Why a fetched stylesheet was not added to the cascade.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum StylesheetRejected {
    /// Its `media=` can never match a screen.
    #[error("media query does not match screen")]
    NeverMatches,
    /// The text is not a stylesheet.
    #[error("stylesheet does not parse: {0}")]
    Unparsable(css::CssError),
}

/// Parses `text` as an author sheet limited to `media`.
///
/// # Errors
///
/// [`StylesheetRejected`] when `media` can never match a screen or `text` is
/// not a stylesheet.
pub fn parse_author_stylesheet(
    text: &str,
    media: &MediaQuery,
) -> Result<StyleSheetSet, StylesheetRejected> {
    if media.is_never() {
        return Err(StylesheetRejected::NeverMatches);
    }
    let mut sheet =
        css::parse_stylesheet(text, Origin::Author).map_err(StylesheetRejected::Unparsable)?;
    sheet.apply_media(media);
    tracing::debug!(
        rules = sheet.rules().count(),
        notes = sheet.notes().len(),
        bytes = text.len(),
        "stylesheet parsed"
    );
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use css::MediaQuery;

    use super::{StylesheetRejected, parse_author_stylesheet};

    #[test]
    fn a_screen_stylesheet_parses_into_its_rules() {
        let sheet = parse_author_stylesheet("p { color: red }", &MediaQuery::always()).unwrap();

        assert_eq!(sheet.rules().count(), 1);
    }

    #[test]
    fn a_never_matching_media_is_rejected_before_parsing() {
        let rejected = parse_author_stylesheet("p {}", &MediaQuery::never());

        assert_eq!(rejected.unwrap_err(), StylesheetRejected::NeverMatches);
    }
}
