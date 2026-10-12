# BLISS-SUBDOC-1 — propagate leftover wheel scroll from a sub-document to its parent

**Status:** Backlog
**Filed:** 2026-10-12, from surfer-browser `SURF-ENGINE-2` / `SURF-SUBDOC-1`

## Problem

A wheel event on a sub-document (`DocumentMutator::set_sub_document`) scrolls the hovered node or the
sub-document viewport, and any leftover delta is dropped (`packages/bliss-dom/src/document.rs` ~1691:
`// TODO: propagate remaining scroll to parent`). super-surfer plans to host every web page as a
sub-document of its chrome document, so scrolling past the end of an embedded page/container stops dead
instead of chaining outward as browsers do.

## Done when

- [ ] Remaining delta from a sub-document is returned to the parent, which continues on its hovered/ancestor
      scroll container.
- [ ] `overscroll-behavior: contain | none` on the sub-document root (or host element) stops chaining.
- [ ] Test: nested sub-document at bottom limit → parent scrolls by the remainder.
