# BLISS-AINATIVE-1 — agents modify the shell/DOM: a live control surface

**Status:** Backlog
**Filed:** 2026-11-06, from arniko `docs/RUNE_ENGINE_BORROW.md` + BLISS-HEADLESS-1.

## Idea

Rune makes an app agent-observable and agent-drivable **headlessly** (element-tree dump,
scripted input). The natural next step is to let an agent **modify** a running shell/DOM —
restyle, retarget, hot-reload CSS/HTML — from outside the process, closing the loop
**observe → decide → mutate → re-observe**.

Everything needed already exists as primitives; the gap is a channel and a few conveniences.

## What exists today

| Half | API |
| --- | --- |
| Read | `BaseDocument::document_tree_json` (BLISS-HEADLESS-1) |
| Drive | `EventDriver::synthesize_*` (BLISS-HEADLESS-1) |
| Time | `BaseDocument::set_clock_ms` (BLISS-HEADLESS-1) |
| Write | `DocumentMutator`, and now `BaseDocument::apply_mutations` + `Mutation` |
| Sandbox | `PolicyDomController` |
| Stylesheets | `add_stylesheet_for_node` / `upsert_stylesheet_for_node` |

## Proposed

1. **Serializable mutation ops** — *done*: `Mutation` (set/remove attribute, set style,
   set text, remove) + `BaseDocument::apply_mutations`; plain data so it maps from JSON.
2. **Live control channel** — a **localhost-only** socket in the host (bliss-shell, or a small
   dev binary) that accepts a JSON op list (`apply_mutations` + `resolve`) and returns
   `document_tree_json`. Dev-only: behind a feature and an auth token, never on in a shipped
   app. Precedents: `procwatch`'s loopback JSON; Rune's `--hot` / `rune-dev`.
3. **CSS / HTML hot-reload** — watch an author stylesheet or the shell document, re-inject via
   the stylesheet API and re-resolve; the shell regenerates its CSS (surfer's `cached_css`) and
   pushes it. `resolve` already re-styles from the current tree.
4. **Safety** — capability-gate the channel (localhost + token); route through
   `PolicyDomController` where the content is untrusted; keep it deterministic with the virtual
   clock.

## Done when

- [ ] The dev-only control channel round-trips: apply a mutation list → resolve → dump.
- [ ] A stylesheet/CSS hot-reload path.
- [ ] Documented authorization + sandbox story; never enabled by default.

## Note

The read/drive/time/write primitives are engine-side (bliss-dom) and shipped; the channel and
hot-reload are host-side and belong with whatever owns the window (bliss-shell / the app).
