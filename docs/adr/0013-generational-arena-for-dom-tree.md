# ADR-0013: Generational Arena and Slot Allocation in DomTree

## Status

Accepted

## Date

2026-08-26

## Context

The DOM tree arena in `DomTree` originally used a flat `Vec<Option<DomNode>>` indexed by primitive `NodeId(u32)`
indices. This design posed structural limitations:

1. **ABA Problem on Slot Reuse**: If a node was deleted and its slot index re-allocated to a new node, stale `NodeId`
   handles held elsewhere (such as style cascades, layout trees, or user scripts) would silently point to the new node,
   leading to data corruption and memory safety bugs.
2. **Repetitive Pre-condition Checking (C-26)**: Mutating operations (`append_child`, `insert_before`, `remove_child`)
   repeatedly invoked `validate_exists` manually on each operand in pairs and triplets.
3. **Absence of Generational Invariants (C-27)**: Arena nodes lacked generational tracking to ensure unforgeable,
   monotonic validity over the lifetime of tree mutations.

## Decision

1. **Generational `NodeId`**: `NodeId` now carries both `index: u32` and `generation: u32`. Construction is handled via
   `NodeId::with_generation(index, generation)` and `NodeId::new(index)` (defaulting to generation 0 for backward
   compatibility).

2. **`Slot<T>` Arena**: Internal storage is modeled as `Vec<Slot<DomNode>>` where:
    - `Slot::Occupied { data: DomNode, generation: u32 }`
    - `Slot::Vacant { next_free: Option<u32>, generation: u32 }` Vacant slots maintain a linked free list tracked by
      `free_head: Option<u32>`.

3. **Monotonic Generation Increment on Deletion**: When a node is deleted, its slot generation increments by 1.
   Re-allocating the slot for a new node inherits this incremented generation, immediately invalidating all stale
   `NodeId` handles.

4. **Multi-Node Resolution (`resolve_all`)**: Introduce `DomTree::resolve_all(&[NodeId]) -> Result<(), DomError>` to
   validate existence and generation match atomically across multiple nodes with the `?` operator.

## Consequences

### Positive

- **Elimination of ABA Problem**: Stale handles cannot access newly allocated nodes at recycled indices.
- **Clean Validation**: Replaces scattered `validate_exists` calls with atomic `resolve_all`.
- **Zero-Allocation Node Recycling**: Freelist reuse allows the arena to avoid vector reallocations during intense DOM
  mutations.

### Negative / Trade-offs

- `NodeId` increases in size from 4 bytes (`u32`) to 8 bytes (`u32 + u32`), which remains negligible for cache and
  memory efficiency.
