# OpenUnity

An independent, clean-room reimplementation of Unity, rebuilt in Rust.

OpenUnity aims to run existing C# gameplay scripts and Unity projects through an
independently implemented native engine, managed API layer, editor, and build
pipeline. The compatibility baseline is **Unity 7000.0.0a8**, build
`c93e369fcf1c`.

This is an early implementation. Full Unity compatibility is the long-term goal;
current support covers bounded parts of the object model, transforms, lifecycle,
and scene serialization. Native systems are written in Rust. Independently
implemented C# assemblies provide the supported gameplay API and .NET hosting.

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
| Editor | Native authoring UI with hierarchy, inspector, project browser, console, transform markers, undo/redo, and independent scene persistence. |

General Unity project import, public SceneManager compatibility, game rendering,
physics, audio, animation, and complete UnityEditor APIs remain unimplemented or
unverified. The editor viewport currently displays authoring markers; it does not
render a game scene. Editor Play mode and general Unity project builds are not
supported yet.

Engine functionality is the current priority. Editor visual parity comes later.

## Build from source

Requirements:

- Rust 1.93.1 (pinned in rust-toolchain.toml) and a native C/C++ linker.
- .NET SDK 8.0.425, with the patch roll-forward policy in [global.json](global.json).
- A graphics-capable desktop session to launch the editor.

On Ubuntu, install the editor's native development dependencies:

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
arguments for its developer hosting and scene-loading commands.

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
