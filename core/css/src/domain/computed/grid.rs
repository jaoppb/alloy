//! Computed-value types for CSS Grid Layout Level 1/2 (CSS Grid L1 §7–§8).
//!
//! Grouped into [`GridStyle`] (`ADR-0010` rule 7) to keep [`crate::domain::computed::style::ComputedStyle`]
//! focused, readable, and `Copy`.

#[path = "grid/area_name.rs"]
pub mod area_name;
#[path = "grid/area_rect.rs"]
pub mod area_rect;
#[path = "grid/auto_flow.rs"]
pub mod auto_flow;
#[path = "grid/fr.rs"]
pub mod fr;
#[path = "grid/gap.rs"]
pub mod gap;
#[path = "grid/placement.rs"]
pub mod placement;
#[path = "grid/style.rs"]
pub mod style;
#[path = "grid/template_areas.rs"]
pub mod template_areas;
#[path = "grid/track_list.rs"]
pub mod track_list;
#[path = "grid/track_size.rs"]
pub mod track_size;

pub use area_name::GridAreaName;
pub use area_rect::GridAreaRect;
pub use auto_flow::GridAutoFlow;
pub use fr::GridFr;
pub use gap::GridGap;
pub use placement::{GridLine, GridLineName, GridPlacement, GridSpan};
pub use style::GridStyle;
pub use template_areas::GridTemplateAreas;
pub use track_list::TrackList;
pub use track_size::{MaxTrackBreadth, MinTrackBreadth, TrackSize};
