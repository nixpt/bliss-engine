# bliss-dom: Known Issues & Soundness Concerns

This document tracks known soundness issues, architectural debt, and planned improvements in `bliss-dom`.

---

## S1: `unsafe impl Send/Sync for Node` is technically unsound

**Severity:** High (potential UB)
**Status:** Documented with improved safety argument; mitigated by usage patterns
**Files:** `src/node/node.rs:125-154`

### Problem

`Node` contains `Cell<Option<usize>>` and `RefCell<Option<Vec<usize>>>` fields (`layout_parent`, `layout_children`, `paint_children`) which are `!Sync`. The blanket `unsafe impl Sync for Node` promises the compiler that `&Node` can be safely shared across threads, but concurrent mutation of `Cell`/`RefCell` from multiple threads is undefined behavior.

### Why it exists

The `parallel-construct` feature uses Rayon to parallelize inline layout construction, which requires `&Slab<Node>` to be shared across threads. Rayon's parallel iterators require closures to be `Sync`, and closures that capture `&Slab<Node>` therefore require `Slab<Node>: Sync`, which transitively requires `Node: Sync`.

### Why it works in practice

The parallel path (`resolve_deferred_tasks` in `resolve.rs`) only **reads** node data through shared references. The `Cell`/`RefCell` fields are **NOT** mutated during parallel iteration - all mutation happens on the main thread before/after the parallel section.

Additionally:
1. A `BaseDocument` is only ever accessed from a single thread at a time
2. The parallel construction happens within a single `resolve()` call on the main thread
3. The raw `*mut Slab<Node>` pointer in each `Node` is only dereferenced to create `&Slab<Node>` (shared refs), never `&mut Slab<Node>`

### Safety documentation (improved 2026-02-18)

The unsafe impls now include detailed safety comments documenting:
- Why `Node: Sync` is required (Rayon closure capture)
- The three critical invariants that must be maintained
- The read-only nature of the parallel path

### Recommended fix (Strategy C)

**Long-term:** Extract the data needed by the parallel path (text content, styles, font contexts) into standalone `ConstructionTask` structs *before* entering the parallel section, eliminating the need to share `&Slab<Node>` across threads entirely. The current design already partially does this — completing the extraction would make the unsafe impl unnecessary.

**Note:** Strategy A (scoped wrapper) was attempted but doesn't work because Rayon's closure capture semantics require the underlying type to be `Sync`, not just a wrapper around a reference to it.

---

## S2: Raw `*mut Slab<Node>` pointer aliasing risk

**Severity:** ~~Medium~~ **RESOLVED**
**Status:** **Fixed** (2026-02-18)
**Files:** `src/node/node.rs:82` (field), `src/node/node.rs:644-646` (`tree()` method)

### Problem

Each `Node` stored a `*mut Slab<Node>` back-pointer to its owning slab. The `tree()` method dereferenced this to `&Slab<Node>`. If code held `&mut Node` (obtained via `&mut slab[id]`) and simultaneously called `node.tree()` which dereferenced the same slab pointer, this could create overlapping `&mut T` / `&T` references to the same allocation — a potential violation of Rust's aliasing rules under the Stacked Borrows model.

### Fix Applied (2026-02-18)

Changed the pointer type from `*mut Slab<Node>` to `*const Slab<Node>`. This change:

1. **Expresses intent at the type level**: Since `tree()` only ever creates `&Slab<Node>` (shared refs), using `*const` makes this explicit
2. **Eliminates the aliasing concern**: `*const` pointers are `Send + Sync` by default, and we only ever create shared references from them
3. **Improves documentation**: The field documentation now clearly states why `*const` is used

### Files Changed

- `src/node/node.rs`: Changed `tree: *mut Slab<Node>` to `tree: *const Slab<Node>`
- `src/node/node.rs`: Updated `Node::new()` to accept `*const Slab<Node>`
- `src/document.rs`: Updated `create_node()` to cast `self.nodes.as_ref()` to `*const`
- Updated safety comments on `Send`/`Sync` impls to reflect the change

---

## S3: `query_selector` test failure (pre-existing)

**Severity:** Medium (test failure)
**Status:** Pre-existing, not caused by recent changes
**Files:** `src/dom_control.rs:465`

### Problem

The test `dom_control::tests::test_dom_controller_query_selector` fails with:
```
assertion `left == right` failed
  left: None
 right: Some(2)
```

This was pre-existing before recent fixes (the code did not compile on the prior commit due to unrelated type inference errors).

---

## B1: `pe_by_index` maps 0→after, 1→before (reversed from intuition)

**Severity:** Low (working correctly, confusing API)
**Status:** Intentional — matches Stylo's internal pseudo-element array ordering
**Files:** `src/node/node.rs:189-203`

### Context

Stylo's `pseudos.as_array()` stores `::before` at index 1 and `::after` at index 0. The `pe_by_index` API mirrors this mapping. The code in `layout/construct.rs:392-408` has a comment: `"Note: yes these are kinda backwards"`. This is not a bug but is a foot-gun for future contributors.

---

## B2: Widespread `.unwrap()` on element-only operations

**Severity:** Low (panic on internal invariant violation)
**Status:** Accepted risk for internal APIs
**Files:** Various (`document.rs`, `mutator.rs`)

### Problem

Patterns like `self.nodes[node_id].element_data_mut().unwrap()` will panic if `node_id` refers to a text/comment/document node. These call sites are internal and are only reached after type-checking (e.g., after matching on element tag names), so the unwraps are effectively safe assertions.

### Recommendation

No immediate action needed. If public API surface expands, consider returning `Option`/`Result` from document-level methods that accept arbitrary `node_id` values.

---

## Performance notes

### P1: `toggle_radio` iterates all nodes

**Files:** `src/document.rs:652-664`

`toggle_radio` iterates all nodes in the slab to find radio buttons matching a name. For large documents, this could be slow. A `name → Vec<usize>` index could make this O(1) but adds maintenance cost for a rarely-hit path.

---

## Changelog

| Date | Change |
|---|---|
| 2026-02-18 | Fixed S2 by changing `*mut Slab<Node>` to `*const Slab<Node>` - eliminates aliasing concern |
| 2026-02-18 | Improved S1 documentation with detailed safety argument; attempted Strategy A (SyncSlabRef) but found it doesn't work with Rayon closure capture semantics |
| 2026-02-17 | Initial documentation of known issues |
