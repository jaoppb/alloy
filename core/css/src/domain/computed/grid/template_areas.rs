//! First-class collection of named template areas (CSS Grid L1 §7.3).

use core::fmt;

use super::area_name::GridAreaName;
use super::area_rect::GridAreaRect;

/// First-class collection of named template areas (CSS Grid L1 §7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridTemplateAreas {
    cells: [Option<GridAreaName>; Self::CAPACITY],
    rows: u8,
    columns: u8,
}

impl Default for GridTemplateAreas {
    fn default() -> Self {
        Self::none()
    }
}

impl GridTemplateAreas {
    /// Maximum cell count supported in inline fixed capacity.
    pub const CAPACITY: usize = 64;

    /// Empty named areas — CSS `none`.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            cells: [None; Self::CAPACITY],
            rows: 0,
            columns: 0,
        }
    }

    /// Whether named template areas are absent (`none`).
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.rows == 0
    }

    /// Number of explicit template rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        usize::from(self.rows)
    }

    /// Number of explicit template columns.
    #[must_use]
    pub fn column_count(&self) -> usize {
        usize::from(self.columns)
    }

    /// Validates rectangular invariants and constructs template areas. `None`
    /// for a ragged matrix, a non-rectangular area, or more than
    /// [`Self::CAPACITY`] cells.
    #[must_use]
    pub fn from_matrix(matrix: &[Vec<Option<GridAreaName>>]) -> Option<Self> {
        if matrix.is_empty() {
            return Some(Self::none());
        }
        let columns = matrix.first()?.len();
        if columns == 0 || matrix.iter().any(|row| row.len() != columns) {
            return None;
        }
        if matrix.len().saturating_mul(columns) > Self::CAPACITY {
            return None;
        }
        let mut cells = [None; Self::CAPACITY];
        for (slot, cell) in cells.iter_mut().zip(matrix.iter().flatten()) {
            *slot = *cell;
        }
        let areas = Self {
            cells,
            rows: u8::try_from(matrix.len()).ok()?,
            columns: u8::try_from(columns).ok()?,
        };
        areas.all_areas_rectangular().then_some(areas)
    }

    fn cell(&self, row: usize, column: usize) -> Option<GridAreaName> {
        if row >= self.row_count() || column >= self.column_count() {
            return None;
        }
        let index = row
            .saturating_mul(self.column_count())
            .saturating_add(column);
        self.cells.get(index).copied().flatten()
    }

    /// The occupied cells, row-major.
    fn occupied_cells(&self) -> &[Option<GridAreaName>] {
        let total = self.row_count().saturating_mul(self.column_count());
        self.cells.get(..total).unwrap_or(&[])
    }

    fn all_areas_rectangular(&self) -> bool {
        self.occupied_cells()
            .iter()
            .flatten()
            .all(|name| self.is_area_rectangular(*name))
    }

    /// An area is valid when its bounding box holds exactly its own cells
    /// (CSS Grid L1 §7.3: "a single filled-in rectangle").
    fn is_area_rectangular(&self, name: GridAreaName) -> bool {
        let Some(rect) = self.area_rect(name) else {
            return false;
        };
        let expected_cells = usize::try_from(rect.row_span())
            .unwrap_or(0)
            .saturating_mul(usize::try_from(rect.column_span()).unwrap_or(0));
        self.count_name_occurrences(name) == expected_cells
    }

    fn count_name_occurrences(&self, name: GridAreaName) -> usize {
        self.occupied_cells()
            .iter()
            .filter(|cell| **cell == Some(name))
            .count()
    }

    /// Finds the 1-based bounding box of an area name. The lookup is
    /// case-sensitive, as area names are `<custom-ident>`s (CSS Values 4 §4.2).
    #[must_use]
    pub fn find_area(&self, name: &str) -> Option<GridAreaRect> {
        GridAreaName::new(name).and_then(|target| self.area_rect(target))
    }

    fn area_rect(&self, target: GridAreaName) -> Option<GridAreaRect> {
        let mut positions = self.positions_of(target);
        let first = positions.next()?;
        positions
            .fold(CellBounds::at(first), CellBounds::including)
            .to_rect()
    }

    /// The 0-based `(row, column)` of every cell named `target`.
    fn positions_of(&self, target: GridAreaName) -> impl Iterator<Item = (usize, usize)> + '_ {
        let columns = self.column_count().max(1);
        self.occupied_cells()
            .chunks(columns)
            .enumerate()
            .flat_map(move |(row, cells)| {
                cells
                    .iter()
                    .enumerate()
                    .filter(move |(_, cell)| **cell == Some(target))
                    .map(move |(column, _)| (row, column))
            })
    }
}

/// The 0-based, inclusive bounding box of the cells seen so far.
#[derive(Clone, Copy)]
struct CellBounds {
    first_row: usize,
    last_row: usize,
    first_column: usize,
    last_column: usize,
}

impl CellBounds {
    /// The bounds of a single cell.
    const fn at((row, column): (usize, usize)) -> Self {
        Self {
            first_row: row,
            last_row: row,
            first_column: column,
            last_column: column,
        }
    }

    /// These bounds grown to cover one more cell.
    fn including(self, (row, column): (usize, usize)) -> Self {
        Self {
            first_row: self.first_row.min(row),
            last_row: self.last_row.max(row),
            first_column: self.first_column.min(column),
            last_column: self.last_column.max(column),
        }
    }

    /// The 1-based grid lines around these cells: a cell at 0-based row `r`
    /// sits between lines `r + 1` and `r + 2`.
    fn to_rect(self) -> Option<GridAreaRect> {
        let line = |index: usize, offset: u32| u32::try_from(index).ok()?.checked_add(offset);
        Some(GridAreaRect::new(
            line(self.first_row, 1)?,
            line(self.first_column, 1)?,
            line(self.last_row, 2)?,
            line(self.last_column, 2)?,
        ))
    }
}

impl fmt::Display for GridTemplateAreas {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            return formatter.write_str("none");
        }
        for row in 0..self.row_count() {
            if row > 0 {
                formatter.write_str(" ")?;
            }
            self.fmt_row(formatter, row)?;
        }
        Ok(())
    }
}

impl GridTemplateAreas {
    fn fmt_row(&self, formatter: &mut fmt::Formatter<'_>, row: usize) -> fmt::Result {
        formatter.write_str("\"")?;
        for column in 0..self.column_count() {
            if column > 0 {
                formatter.write_str(" ")?;
            }
            match self.cell(row, column) {
                Some(name) => write!(formatter, "{name}")?,
                None => formatter.write_str(".")?,
            }
        }
        formatter.write_str("\"")
    }
}
