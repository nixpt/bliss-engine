# BLISS-SUBDOC-2 — worker-owned / worker-fed sub-documents (`Document` is `!Send`)

**Status:** Backlog
**Filed:** 2026-10-12, from surfer-browser `SURF-ENGINE-2` / `SURF-SUBDOC-1`

## Problem

`trait Document` (`packages/bliss-dom/src/document.rs` ~132) is not `Send`, so a sub-document must be created
and live on the UI thread — its style, layout and paint run there. super-surfer runs each tab's page on a
worker (network, parse, JS, and today style/layout/paint to a texture). Embedding pages as sub-documents
without losing that isolation needs one of:

1. **Proxy sub-document** — a UI-thread `Document` that forwards events to a worker over a channel and paints
   a worker-produced scene/display list; hit-testing via a worker-provided hit-test map so hover/cursor stay
   synchronous.
2. **`Send` documents** — make `BaseDocument` (or a frozen snapshot) `Send` so a worker builds the DOM and hands
   it over; script mutations go through a command queue.
3. **UI-thread document, parallel work** — keep it on the UI thread but run stylo's parallel traversal
   (bliss currently calls `traverse_dom(&traverser, token, None)`, i.e. no style thread pool) and parallel layout.

## Done when

- [ ] Option chosen and written up in `docs/`, with frame-time numbers for a large page (e.g. Wikipedia) for
      options 1 and 3.
- [ ] A minimal API to embed worker-produced content as a sub-document that still receives the parent's
      input forwarding, focus and scroll.
