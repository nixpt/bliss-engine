# BLISS-STYLE-1 — `set_style_property` does not restyle an already-styled element

**Status:** In review
**Filed:** 2026-11-06, from the reviews of bliss-engine #18 and arniko #29

## Problem

`BaseDocument::set_style_property` and `remove_style_property` edit the element's declaration block
but set no restyle hint and mark no ancestor dirty, so the next `resolve` skips the element. Anything
driving style through them per frame (animation, live bindings) changes nothing on screen until an
unrelated restyle happens to reach that subtree. `DocumentMutator`'s versions delegate here, and so
does `PolicyDomController`. `DomController for BaseDocument` already set a hint and was not affected.

## Fix

Call `Node::set_restyle_hint(RestyleHint::RESTYLE_SELF)` after the write in both methods.
`RESTYLE_SELF` is enough because only this element's inline style changed; stylo cascades inherited
changes to children. No parent hint, so per-frame writes do not restyle siblings.

## Acceptance

- [x] Set and remove take effect on the next `resolve`, on parsed HTML (computed style and layout)
- [x] Repeated per-frame writes each take effect
- [x] An inherited change reaches children
- [x] Tests in `packages/bliss-html/tests/style_property.rs`, failing before the fix

## Follow-up

arniko depends on bliss-dom from crates.io, so it picks this up only after a release.
