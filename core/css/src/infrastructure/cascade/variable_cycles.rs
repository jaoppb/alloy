//! Which custom properties sit on a `var()` reference cycle (CSS Variables
//! L1 §2.3).
//!
//! The spec makes **every** property in a cycle invalid at computed-value
//! time, a reference inside a `var()` fallback counting as an edge too. The
//! substitution's own stack only sees the cycle it happens to walk through,
//! and only from the variable it happened to start at, so a fallback could
//! hide a member and the answer would depend on how the names sort. This
//! module finds the strongly connected components of the reference graph
//! instead — Tarjan's algorithm, iterative so a long hostile chain of
//! references cannot exhaust the stack — and so names every member up front.

use std::collections::BTreeMap;

use crate::domain::computed::variables::{CustomPropertiesMap, VariableName};
use crate::infrastructure::cascade::variable_values::referenced_variables;

/// The custom properties that sit on a reference cycle, each with the cycle
/// it belongs to.
#[derive(Debug, Default)]
pub(super) struct CyclicVariables {
    component_of: BTreeMap<VariableName, usize>,
    components: Vec<Vec<VariableName>>,
}

impl CyclicVariables {
    /// Every cycle reachable from `roots` through the references of `map`.
    pub(super) fn reachable_from<'roots>(
        map: &CustomPropertiesMap,
        roots: impl IntoIterator<Item = &'roots VariableName>,
    ) -> Self {
        let mut finder = ComponentFinder::new(map);
        for root in roots {
            finder.visit_from(root);
        }
        finder.cyclic
    }

    /// The cycle `name` belongs to, as the names it runs through, or `None`
    /// when `name` is on no cycle.
    pub(super) fn cycle_through(&self, name: &VariableName) -> Option<Vec<String>> {
        let component = self.components.get(*self.component_of.get(name)?)?;
        let members = component.iter().map(|member| member.as_str().to_owned());
        Some(members.chain([name.as_str().to_owned()]).collect())
    }

    fn record(&mut self, component: Vec<VariableName>) {
        let index = self.components.len();
        for member in &component {
            self.component_of.insert(member.clone(), index);
        }
        self.components.push(component);
    }
}

/// Where Tarjan's walk stands on one variable.
#[derive(Clone, Copy)]
struct Visit {
    index: usize,
    low_link: usize,
    on_stack: bool,
}

/// One variable being walked: its references, and how many are followed.
struct Frame {
    name: VariableName,
    successors: Vec<VariableName>,
    next: usize,
}

impl Frame {
    fn next_successor(&mut self) -> Option<VariableName> {
        let successor = self.successors.get(self.next)?.clone();
        self.next = self.next.saturating_add(1);
        Some(successor)
    }

    fn refers_to_itself(&self) -> bool {
        self.successors.contains(&self.name)
    }
}

/// Tarjan's strongly-connected-components walk over the references of one
/// map, with an explicit stack of [`Frame`]s instead of recursion.
struct ComponentFinder<'map> {
    map: &'map CustomPropertiesMap,
    visits: BTreeMap<VariableName, Visit>,
    stack: Vec<VariableName>,
    next_index: usize,
    cyclic: CyclicVariables,
}

impl<'map> ComponentFinder<'map> {
    fn new(map: &'map CustomPropertiesMap) -> Self {
        Self {
            map,
            visits: BTreeMap::new(),
            stack: Vec::new(),
            next_index: 0,
            cyclic: CyclicVariables::default(),
        }
    }

    fn visit_from(&mut self, root: &VariableName) {
        if self.visits.contains_key(root) {
            return;
        }
        let mut frames: Vec<Frame> = self.open(root).into_iter().collect();
        while let Some(top) = frames.last_mut() {
            let successor = top.next_successor();
            self.advance(&mut frames, successor);
        }
    }

    /// Follows the top frame's next reference, or closes the frame when it
    /// has none left.
    fn advance(&mut self, frames: &mut Vec<Frame>, successor: Option<VariableName>) {
        let Some(successor) = successor else {
            self.close(frames);
            return;
        };
        if let Some(visit) = self.visits.get(&successor).copied() {
            self.link_to_visited(frames, visit);
            return;
        }
        frames.extend(self.open(&successor));
    }

    /// Starts walking `name`, or `None` when the map does not define it — an
    /// undefined variable has no references, so it closes no cycle.
    fn open(&mut self, name: &VariableName) -> Option<Frame> {
        let value = self.map.get(name)?;
        let index = self.next_index;
        self.next_index = index.saturating_add(1);
        let visit = Visit {
            index,
            low_link: index,
            on_stack: true,
        };
        self.visits.insert(name.clone(), visit);
        self.stack.push(name.clone());
        let successors = referenced_variables(value.as_str())
            .filter(|successor| self.map.contains(successor))
            .collect();
        Some(Frame {
            name: name.clone(),
            successors,
            next: 0,
        })
    }

    /// A reference to a variable already walked: if it is still on the stack
    /// it closes a cycle through the top frame.
    fn link_to_visited(&mut self, frames: &[Frame], visit: Visit) {
        let Some(top) = frames.last() else {
            return;
        };
        if visit.on_stack {
            self.lower_low_link(&top.name, visit.index);
        }
    }

    /// Finishes the top frame: hands its low-link to its parent and, when it
    /// roots a component, pops that component off the stack.
    fn close(&mut self, frames: &mut Vec<Frame>) {
        let Some(frame) = frames.pop() else {
            return;
        };
        let Some(visit) = self.visits.get(&frame.name).copied() else {
            return;
        };
        if let Some(parent) = frames.last() {
            self.lower_low_link(&parent.name, visit.low_link);
        }
        if visit.low_link == visit.index {
            self.pop_component(&frame);
        }
    }

    fn lower_low_link(&mut self, name: &VariableName, candidate: usize) {
        if let Some(visit) = self.visits.get_mut(name) {
            visit.low_link = visit.low_link.min(candidate);
        }
    }

    /// Pops the component `root` roots and records it when it is a cycle:
    /// more than one member, or one that refers to itself.
    fn pop_component(&mut self, root: &Frame) {
        let start = self
            .stack
            .iter()
            .rposition(|member| *member == root.name)
            .unwrap_or(0);
        let component = self.stack.split_off(start);
        for member in &component {
            self.leave_stack(member);
        }
        if component.len() > 1 || root.refers_to_itself() {
            self.cyclic.record(component);
        }
    }

    fn leave_stack(&mut self, name: &VariableName) {
        if let Some(visit) = self.visits.get_mut(name) {
            visit.on_stack = false;
        }
    }
}
