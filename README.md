# Nexa Framework

Nexa is a production-grade, cross-platform UI framework written in Rust. It prioritizes performance, deterministic execution, and direct GPU rendering for mobile platforms.

## Core Features

- **Fine-Grained Reactivity**: Signal-based architecture with `smallvec` optimizations.
- **Topological Scheduler**: Deterministic execution order based on dependency depth.
- **Arena-Based VDOM**: Fast memory allocation using `slotmap`.
- **Streaming SSR**: Incremental HTML emission without buffering.
- **GPU-Native Mobile**: `wgpu`-based renderer for Android and iOS.
- **Cross-Platform**: Web (WASM), Desktop (Winit), Mobile (Native).

## Getting Started

To create a new project:

```bash
nexa new my-app
cd my-app
nexa dev
```

## Structure

- `nexa-core`: The heart of the framework (VDOM, Runtime).
- `nexa-signals`: Reactivity engine.
- `nexa-scheduler`: Execution scheduler.
- `nexa-renderer-gpu`: Native GPU renderer.
- `nexa-ssr`: Streaming server-side rendering.

## License

MIT
