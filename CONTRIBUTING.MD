# Contributing to Bliss Engine 🌌

Welcome to the **Exosphere** developer community!

**Bliss** is an agent-native HTML/CSS rendering engine. Unlike traditional browsers, Bliss is designed as a modular library that can be integrated into high-performance, polyglot agent runtimes.

We are a hard fork of [Dioxus Labs' Blitz](https://github.com/DioxusLabs/blitz), and we focus on features like:
- **Capability-Gated Rendering**
- **Polyglot Scripting (Beyond V8/JavaScript)**
- **Simulation-Synchronized State**
- **Non-Blocking Messaging Architecture**

---

## 🚀 Development Environment

### Protocol
1. **Check Context**: Read the Exosphere `README.md` and `BLISS_MANIFESTO.md` first.
2. **Follow The Constitution**: All code must adhere to the secure capability safety model.

### Linux Requirements
Ensure you have the following system dependencies installed for development:
```sh
sudo apt-get update
sudo apt-get install \
    libasound2-dev \
    libatk1.0-dev \
    libgtk-3-dev \
    libudev-dev \
    libpango1.0-dev \
    libxdo-dev
```

### Build & Test
We use the root **Exosphere Makefile** for high-level tasks, but you can build standalone packages:
```sh
cargo build -p bliss-dom
cargo test -p bliss-traits
```

---

## 🌉 Upstream Policy

Bliss is an independent hard fork and does not track upstream Blitz.
- **Fixes land here.** Fix bugs in this repo. Porting a fix *from* upstream Blitz is welcome; cite the upstream commit in the message and keep its notices.
- **No upstream submissions from this project.** Do not open issues or PRs against DioxusLabs, Servo or Linebender repos on behalf of Bliss, and never from an agent. If a fix belongs upstream, a human maintainer decides, self-reviews and files it.
- **Dependencies.** Taffy, Stylo, Parley, Vello and anyrender are crates.io dependencies; bump them on purpose, one at a time, with a changelog note.
- **Divergence.** The `ScriptEngine` trait, capability safety and simulation hooks are core Bliss features.

---
*Exosphere: Architecting the Post-Human Interface.*
