# Sargo's Pizza

A space racing game written in Rust on [wgpu](https://wgpu.rs). There is no game engine underneath: the renderer, shader and pipeline management, physics and input handling are written from scratch, and `bevy_ecs` supplies only the entity-component system.

You play an automated delivery drone that has to get orders across the galaxy before the clock runs out.

**Status:** work in progress. The renderer, ship controls and gravity are implemented; the delivery gameplay itself is still to come.

## What's implemented

- wgpu renderer with an instanced solid pass, planet shadows, bloom and post-processing
- Planets, sun and star field shaded procedurally in WGSL
- Shader and render pipeline manager (`src/shader_manager`)
- Ship controller with momentum-based speed, banking and a smoothed chase camera
- Pairwise gravity between bodies
- Debug gizmos
- Game systems scheduled with `bevy_ecs`

## Running

```sh
cargo run --release
```

A Nix flake is included, so `nix develop` gives a shell with the toolchain.

## Design notes

The gameplay and art direction are written up in [`Game Notes/`](Game%20Notes).
