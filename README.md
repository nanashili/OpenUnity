# OpenUnity

An independent, clean-room reimplementation of Unity, rebuilt in Rust.

OpenUnity aims to run existing C# gameplay scripts and Unity projects through an
independently implemented native engine, managed API layer, editor, and build
pipeline. The compatibility baseline is **Unity 7000.0.0a8**, build
`c93e369fcf1c`.

This is an early implementation. Full Unity compatibility is the long-term goal;
current support covers bounded parts of the object model, transforms, lifecycle,
scene serialization, rendering, and manual physics. Native systems are written
in Rust. Independently implemented C# assemblies provide the supported gameplay
API and .NET hosting.

[Build status and downloads](https://github.com/nanashili/OpenUnity/actions/workflows/build.yml)

## Current functionality

| Area | Current scope |
| --- | --- |
| Objects | Runtime-owned identities, object lifetime, supported destruction and equality behavior. |
| GameObjects and components | Component attachment and lookup, enabled/activity state, and supported MonoBehaviour lifecycle callbacks. |
| Transforms | Parent/child relationships and supported local/world transform operations. |
| C# gameplay | Independent .NET hosting and a growing subset of UnityEngine APIs. |
| Scenes and assets | Bounded Unity text-scene import, metadata/GUID indexing, and supported serialized fields and object references. |
| Project compilation | Explicit authored C# source compilation and headless scene launch. |
| Rendering | Offscreen orthographic triangle rendering with explicit mesh/material bindings, depth testing, and GPU readback. Flat 2D and 3D-depth samples have run on Metal. |
| Physics | The legacy manual-motion slice supports up to 256 collider-free 3D dynamic bodies per runtime, with scripted translation steps and independent native query volumes. A separate runtime-owned collision scene supports bounded 2D boxes and 3D cuboids, gravity, contact response, fixed-phase scheduling, Transform publication, and deterministic removal. |
| Animation and audio | Independent native clip/state sampling and offline audio mixing; general Unity API and content compatibility remain unverified. |
| Editor | Native authoring UI with hierarchy, inspector, project browser, console, transform markers, undo/redo, and independent scene persistence. |

General Unity project import, public SceneManager compatibility, perspective
rendering, automatic Camera/MeshRenderer runtime import, managed Physics2D and
Collider APIs, and complete UnityEditor APIs remain unimplemented or
unverified. Bounded Camera property and MeshFilter/MeshRenderer reference
extraction are available, as described below. Current scene rendering uses a
separate bounded render manifest. The editor viewport displays authoring
markers; Editor Play mode and general Unity project builds remain unsupported.

Native query volumes are a separate foundation with a 256-volume limit per
runtime. They do not follow Transforms or expose Unity Collider/Physics.Raycast
APIs, and have no verified Unity query compatibility.

Multi-body motion has passed native integration, C ABI, and standalone C#
capacity/lifetime checks. Failed batch steps preserve body and Transform state.
This collider-free manual-motion slice is separate from collision scenes; it
does not provide collision response or automatic scheduling.

Native collision scenes use real Rapier 2D/3D solvers and are stepped by the
runtime on an active fixed phase or through an explicit caller step. The
bounded profile admits up to 256 2D bodies or 128 3D bodies, limited to dynamic
boxes/cuboids and fixed boxes/cuboids. It has no collision-event API, filters,
sensors, joints, or managed Unity collider behavior, and Unity parity remains
unverified. Run `--physics-scene <2d|3d> <steps:60..600> <width> <height>
<hardware|software> <fresh-output-dir>` to capture an independently authored
falling-body scene. The command writes initial, middle, and final PPM GPU
readbacks with solver snapshots in `physics-scene.json` and renderer details in
`receipt.json`.

The native asset catalog resolves up to 256 explicit GUID/local-file-ID bindings
to typed meshes and unlit materials. Scene reference extraction separately retains
MeshFilter/MeshRenderer ownership and ordered asset pointers within the supported
text grammar. Automatic scene-to-renderer binding and Unity importer-generated
subobject IDs remain unsupported.

Engine functionality is the current priority. Editor visual parity comes later.

## Build from source

Requirements:

- Rust 1.93.1 (pinned in rust-toolchain.toml) and a native C/C++ linker.
- .NET SDK 8.0.425, with the patch roll-forward policy in [global.json](global.json).
- A graphics-capable desktop session to launch the editor or live player.

On Ubuntu, install the editor and live player's native development dependencies:

```sh
sudo apt-get update
sudo apt-get install --no-install-recommends -y \
  pkg-config libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libxinerama-dev libxkbcommon-dev libwayland-dev libegl1-mesa-dev
```

From the repository root:

```sh
cargo build --workspace --release --locked
dotnet build managed/Hosting/OpenUnity.ManagedHosting.csproj --configuration Release
cargo run -p openunity-player --release --locked -- --self-check
cargo run -p openunity-editor --release --locked
```

The player self-check exercises native object identity and lifetime. It is an
infrastructure check, not a Unity compatibility test. Run the player without
arguments or with `--help` for its developer hosting, scene-loading, and
`--render-scene`, and `--physics-scene` commands. The render route loads one
managed scene session and captures its transforms. The physics route runs a
bounded native 2D or 3D collision scene. Both write PPM frames and a receipt
containing the actual adapter, draw counts, readback status, and cleanup
results.

For an interactive native window, run
`cargo run -p openunity-player -- --live-physics-scene <2d|3d> <hardware|software> [--frames <count>]`.
The route opens a 960x640 window and presents directly to its WGPU surface.
Space pauses or resumes fixed-step simulation; Escape or the window close
control exits. The optional positive frame count exits after that many
successful present requests. The final JSON summary reports present requests,
simulation steps, body state, and collision-body cleanup; a present request
does not imply GPU completion. The scene uses the current Rapier-backed native
2D/3D collision solver; Unity physics parity remains unverified. Its 3D view
uses a fixed oblique orthographic camera. This route runs the built-in scene
only. The live surface path has been runtime-verified on macOS Metal only;
other desktop window backends remain unverified.

The default build uses independent fallback fonts and UI graphics. A Unity
installation, extracted Unity resources, or proprietary runtime components are
not required to build or run it. C# gameplay execution requires a compatible
independent .NET runtime and the managed assemblies.

## Platform builds

GitHub Actions builds release artifacts on all six current desktop targets:

| Platform | Architectures |
| --- | --- |
| Linux | x64, ARM64 |
| Windows | x64, ARM64 |
| macOS | Intel x64, Apple Silicon ARM64 |

Every push to `main`, pull request, and manual workflow run builds the native
workspace and managed assemblies, runs the native player self-check, and uploads
an archive for each target. Each archive contains the editor, player, native
bridge, CLI, and managed assemblies. Platform runtime dependencies are not bundled.
Artifacts are retained for 14 days; select a completed run on the
[Actions page](https://github.com/nanashili/OpenUnity/actions/workflows/build.yml)
to download them.

These checks establish build and self-check coverage, not complete gameplay or
GUI compatibility on each platform. Mobile, browser, and console builds are not
available yet.

## Repository layout

| Path | Responsibility |
| --- | --- |
| [crates](crates) | Explicit engine owners: math, identifiers, objects, serialization, scenes, assets, managed hosting and runtime. |
| [managed](managed) | Independently implemented managed API assemblies and hosting layer. |
| [apps/player](apps/player) | Thin standalone player entrypoint. |
| [apps/cli](apps/cli) | Shared import, script execution and diagnostic commands. |
| [apps/editor](apps/editor) | Desktop authoring application and editor-only services under core/. |
| [managed/Engine.Interop](managed/Engine.Interop) | Explicit managed-to-native interface. |
| [build-tools](build-tools) | Explicit authored-project compilation tool. |

## Implementation approach

Research and implementation are separated through reviewed behavioral contracts
and independently authored fixtures. Decompiled implementation bodies are not
translated into Rust or included in this repository. Proprietary Unity binaries,
fonts, icons, raw research, and internal verification records are excluded from
publication.

Compatibility is expanded incrementally and measured per behavior. A successful
build or a working representative API does not establish full subsystem support.
