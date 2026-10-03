//! [`GridStyle`] — the computed values of CSS Grid Layout.

use super::auto_flow::GridAutoFlow;
use super::gap::GridGap;
use super::placement::GridPlacement;
use super::template_areas::GridTemplateAreas;
use super::track_list::TrackList;
use super::track_size::TrackSize;
use crate::domain::length::Length;

/// The computed values of the CSS Grid Layout module grouped (ADR-0010 rule 7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridStyle {
    template_columns: TrackList,
    template_rows: TrackList,
    template_areas: GridTemplateAreas,
    auto_columns: TrackSize,
    auto_rows: TrackSize,
    auto_flow: GridAutoFlow,
    column_start: GridPlacement,
    column_end: GridPlacement,
    row_start: GridPlacement,
    row_end: GridPlacement,
    gap: GridGap,
}

impl GridStyle {
    /// Every grid property at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            template_columns: TrackList::none(),
            template_rows: TrackList::none(),
            template_areas: GridTemplateAreas::none(),
            auto_columns: TrackSize::Auto,
            auto_rows: TrackSize::Auto,
            auto_flow: GridAutoFlow::Row,
            column_start: GridPlacement::Auto,
            column_end: GridPlacement::Auto,
            row_start: GridPlacement::Auto,
            row_end: GridPlacement::Auto,
            gap: GridGap::ZERO,
        }
    }

    #[must_use]
    pub const fn template_columns(&self) -> &TrackList {
        &self.template_columns
    }

    #[must_use]
    pub const fn template_rows(&self) -> &TrackList {
        &self.template_rows
    }

    #[must_use]
    pub const fn template_areas(&self) -> &GridTemplateAreas {
        &self.template_areas
    }

    #[must_use]
    pub const fn auto_columns(self) -> TrackSize {
        self.auto_columns
    }

    #[must_use]
    pub const fn auto_rows(self) -> TrackSize {
        self.auto_rows
    }

    #[must_use]
    pub const fn auto_flow(self) -> GridAutoFlow {
        self.auto_flow
    }

    #[must_use]
    pub const fn column_start(&self) -> &GridPlacement {
        &self.column_start
    }

    #[must_use]
    pub const fn column_end(&self) -> &GridPlacement {
        &self.column_end
    }

    #[must_use]
    pub const fn row_start(&self) -> &GridPlacement {
        &self.row_start
    }

    #[must_use]
    pub const fn row_end(&self) -> &GridPlacement {
        &self.row_end
    }

    #[must_use]
    pub const fn gap(self) -> GridGap {
        self.gap
    }

    #[must_use]
    pub const fn row_gap(self) -> Length {
        self.gap.row()
    }

    #[must_use]
    pub const fn column_gap(self) -> Length {
        self.gap.column()
    }

    #[must_use]
    pub const fn with_template_columns(self, template_columns: TrackList) -> Self {
        Self {
            template_columns,
            ..self
        }
    }

    #[must_use]
    pub const fn with_template_rows(self, template_rows: TrackList) -> Self {
        Self {
            template_rows,
            ..self
        }
    }

    #[must_use]
    pub const fn with_template_areas(self, template_areas: GridTemplateAreas) -> Self {
        Self {
            template_areas,
            ..self
        }
    }

    #[must_use]
    pub const fn with_auto_columns(self, auto_columns: TrackSize) -> Self {
        Self {
            auto_columns,
            ..self
        }
    }

    #[must_use]
    pub const fn with_auto_rows(self, auto_rows: TrackSize) -> Self {
        Self { auto_rows, ..self }
    }

    #[must_use]
    pub const fn with_auto_flow(self, auto_flow: GridAutoFlow) -> Self {
        Self { auto_flow, ..self }
    }

    #[must_use]
    pub const fn with_column_start(self, column_start: GridPlacement) -> Self {
        Self {
            column_start,
            ..self
        }
    }

    #[must_use]
    pub const fn with_column_end(self, column_end: GridPlacement) -> Self {
        Self { column_end, ..self }
    }

    #[must_use]
    pub const fn with_row_start(self, row_start: GridPlacement) -> Self {
        Self { row_start, ..self }
    }

    #[must_use]
    pub const fn with_row_end(self, row_end: GridPlacement) -> Self {
        Self { row_end, ..self }
    }

    #[must_use]
    pub const fn with_gap(self, gap: GridGap) -> Self {
        Self { gap, ..self }
    }

    #[must_use]
    pub const fn with_row_gap(self, row: Length) -> Self {
        Self {
            gap: self.gap.with_row(row),
            ..self
        }
    }

    #[must_use]
    pub const fn with_column_gap(self, column: Length) -> Self {
        Self {
            gap: self.gap.with_column(column),
            ..self
        }
    }
}
