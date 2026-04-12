---
tags: [architecture, index]
---

# Architecture Reference

Detailed documentation of the alephone-rust project's internal architecture.

## Documents

- [[crate-structure]] -- Workspace layout, each crate's purpose, dependency graph
- [[ecs-architecture]] -- How bevy_ecs is used in marathon-sim: components, resources, queries, tick pipeline
- [[rendering-pipeline]] -- wgpu rendering: shaders, buffers, textures, mesh generation, desktop vs web
- [[game-loop-and-state-machine]] -- Main loop, state transitions, tick accumulation, interpolation
- [[data-flow]] -- How data flows from file parsing through simulation to rendering
