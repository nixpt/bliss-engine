# BLISS-PIPE-1 — the release pipeline ships what CI no longer checks

**Status:** In review
**Filed:** 2026-10-31, from a crates.io audit of the bliss/arniko crates

## Problem

- **CI has been red on `main` since 2026-10-12**, all three jobs (`check + test + clippy`, `msrv`,
  `cargo doc`), for one environment reason: `yeslogic-fontconfig-sys`'s build script panics with
  `Package fontconfig was not found` (pkg-config). `libfontconfig1-dev` is in the package list, but
  `awalsh128/cache-apt-pkgs-action@latest` restores a cached package set in which fontconfig's
  pkg-config file is missing. `publish.yml`'s dry run uses the same step, so a release would fail the
  same way.
- **`cargo doc` doesn't build docs the way docs.rs does.** docs.rs builds each crate alone with its
  `[package.metadata.docs.rs]` features — `all-features = true` for `bliss-shell` and `arniko-bliss`.
  So `bliss-shell` 0.3.1's docs.rs build fails (`unresolved import exo_shell_client`), and nothing
  here noticed.
- **`bliss-shell`'s `exoshell` feature can't compile anywhere outside the old monorepo:** it enables
  `src/exoshell.rs`, which imports `exo_shell_client`, a dependency removed for the crates.io publish
  (the manifest says so). Any consumer enabling it fails to build.
- **`arniko-bliss`'s `documentation` link points at `https://docs.rs/bliss`** — the unrelated
  `bliss` crate whose name clash is why ours is called `arniko-bliss`.

## Done when

- [x] CI and the publish dry run install their system packages with plain `apt-get` (+ `pkg-config`).
- [x] CI runs `cargo doc` per crate with each crate's docs.rs features, as docs.rs does.
- [x] `exoshell` (feature + module) removed from `bliss-shell`.
- [x] `arniko-bliss` → `documentation = "https://docs.rs/arniko-bliss"`.
- [x] Workspace version 0.3.2.
- [ ] CI green on the PR.
- [ ] Released: tag `v0.3.2` (publish.yml), then arniko moves to the 0.3.2 crates.
