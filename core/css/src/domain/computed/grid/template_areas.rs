//! First-class collection of named template areas (CSS Grid L1 §7.3).

use core::fmt;

use super::area_name::GridAreaName;
use super::area_rect::GridAreaRect;

/// First-class collection of named template areas (CSS Grid L1 §7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridTemplateAreas {
    cells: [Option<GridAreaName>; Self::CAPACITY],
    rows: u8,
    cols: u8,
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
            cols: 0,
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
        usize::from(self.cols)
    }

    /// Validates rectangular invariants and constructs template areas.
    #[must_use]
    pub fn from_matrix(matrix: &[Vec<Option<GridAreaName>>]) -> Option<Self> {
        if matrix.is_empty() {
            return Some(Self::none());
        }
        let cols = matrix.first()?.len();
        if cols == 0 || matrix.iter().any(|r| r.len() != cols) {
            return None;
        }
        let rows = matrix.len();
        if rows.saturating_mul(cols) > Self::CAPACITY {
            return None;
        }
        let mut cells = [None; Self::CAPACITY];
        for (r, row) in matrix.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let idx = r.saturating_mul(cols).saturating_add(c);
                if let Some(slot) = cells.get_mut(idx) {
                    *slot = *cell;
                }
            }
        }
        let res = Self {
            cells,
            rows: u8::try_from(rows).ok()?,
            cols: u8::try_from(cols).ok()?,
        };
        if !res.validate_all_areas_rectangular() {
            return None;
        }
        Some(res)
    }

    fn cell(&self, r: usize, c: usize) -> Option<GridAreaName> {
        let rows = usize::from(self.rows);
        let cols = usize::from(self.cols);
        if r >= rows || c >= cols {
            return None;
        }
        let idx = r.saturating_mul(cols).saturating_add(c);
        self.cells.get(idx).copied().flatten()
    }

    fn validate_all_areas_rectangular(&self) -> bool {
        let mut checked: [Option<GridAreaName>; 16] = [None; 16];
        let mut checked_len = 0;
        let rows = usize::from(self.rows);
        let cols = usize::from(self.cols);
        for r in 0..rows {
            for c in 0..cols {
                let Some(name) = self.cell(r, c) else {
                    continue;
                };
                if checked
                    .get(..checked_len)
                    .is_some_and(|s| s.contains(&Some(name)))
                {
                    continue;
                }
                if let Some(slot) = checked.get_mut(checked_len) {
                    *slot = Some(name);
                    checked_len = checked_len.saturating_add(1);
                }
                if !self.is_area_rectangular(name) {
                    return false;
                }
            }
        }
        true
    }

    fn is_area_rectangular(&self, name: GridAreaName) -> bool {
        let Some(rect) = self.find_area(name.as_str()) else {
            return false;
        };
        let r_start = usize::try_from(rect.row_start()).unwrap_or(0);
        let r_end = usize::try_from(rect.row_end()).unwrap_or(0);
        let c_start = usize::try_from(rect.column_start()).unwrap_or(0);
        let c_end = usize::try_from(rect.column_end()).unwrap_or(0);
        for r in r_start..r_end {
            for c in c_start..c_end {
                if self.cell(r.saturating_sub(1), c.saturating_sub(1)) != Some(name) {
                    return false;
                }
            }
        }
        let expected_cells = (usize::try_from(rect.row_span()).unwrap_or(0))
            .saturating_mul(usize::try_from(rect.column_span()).unwrap_or(0));
        self.count_name_occurrences(name) == expected_cells
    }

    fn count_name_occurrences(&self, name: GridAreaName) -> usize {
        let total = usize::from(self.rows).saturating_mul(usize::from(self.cols));
        let slice = self.cells.get(..total).unwrap_or(&[]);
        slice.iter().filter(|&&c| c == Some(name)).count()
    }

    /// Finds 1-based bounding box of an area name.
    #[must_use]
    pub fn find_area(&self, name: &str) -> Option<GridAreaRect> {
        let target = GridAreaName::new(name)?;
        let mut min_r = usize::MAX;
        let mut max_r = 0;
        let mut min_c = usize::MAX;
        let mut max_c = 0;
        let mut found = false;

        let rows = usize::from(self.rows);
        let cols = usize::from(self.cols);
        for r in 0..rows {
            for c in 0..cols {
                if self.cell(r, c) == Some(target) {
                    found = true;
                    min_r = min_r.min(r);
                    max_r = max_r.max(r);
                    min_c = min_c.min(c);
                    max_c = max_c.max(c);
                }
            }
        }
        if !found {
            return None;
        }
        let r1 = u32::try_from(min_r).ok()?.saturating_add(1);
        let c1 = u32::try_from(min_c).ok()?.saturating_add(1);
        let r2 = u32::try_from(max_r).ok()?.saturating_add(2);
        let c2 = u32::try_from(max_c).ok()?.saturating_add(2);
        Some(GridAreaRect::new(r1, c1, r2, c2))
    }
}

impl fmt::Display for GridTemplateAreas {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            return formatter.write_str("none");
        }
        let rows = usize::from(self.rows);
        let cols = usize::from(self.cols);
        for r in 0..rows {
            if r > 0 {
                formatter.write_str(" ")?;
            }
            formatter.write_str("\"")?;
            for c in 0..cols {
                if c > 0 {
                    formatter.write_str(" ")?;
                }
                match self.cell(r, c) {
                    Some(name) => write!(formatter, "{name}")?,
                    None => formatter.write_str(".")?,
                }
            }
            formatter.write_str("\"")?;
        }
        Ok(())
    }
}
