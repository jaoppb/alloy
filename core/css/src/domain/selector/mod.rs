//! The selector value objects of the v0.5 cut
//! (`docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8).
//!
//! The hierarchy is the one CSS Selectors L4 §3 describes, and the reason
//! specificity is a fold and matching is a right-to-left walk:
//!
//! ```text
//! SelectorList  →  ComplexSelector  →  SelectorStep  →  CompoundSelector
//!   "h1, .a"         "nav > li.a"      (Child, li.a)    li + .a + [x] + :hover
//! ```
//!
//! In: type, universal, `.class`, `#id`, `[attr]`, `[attr=v]`, lists; the
//! descendant, `>`, `+` and `~` combinators; `:hover`, `:active`, `:focus`,
//! `:first-child`, `:last-child`, `:nth-child()`. Out, and **refused** by the
//! parser rather than ignored: `:has()`, namespaces, `::before` / `::after`.

pub mod complex;
pub mod component;
pub mod compound;

pub use complex::{Combinator, ComplexSelector, SelectorList, SelectorStep};
pub use component::{AttributeMatch, AttributeSelector, NthFormula, PseudoClass, TypeSelector};
pub use compound::{AttributeSelectors, CompoundSelector, IdentifierList, PseudoClasses};
