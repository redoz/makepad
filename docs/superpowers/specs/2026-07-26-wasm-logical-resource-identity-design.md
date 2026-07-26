# WASM Logical Resource Identity Design

## Goal

Remove build-host filesystem paths from Makepad's WebAssembly resource
system. Native builds retain absolute manifest directories for direct file
loading and live reload, while WebAssembly identifies packaged resources by
crate name and crate-relative path.

This design supersedes
`2026-07-26-resource-path-normalization-design.md`. That design repaired
separator handling while retaining the build-host path as a WebAssembly
runtime identity; this design removes that dependency instead.

## Scope

This change covers Makepad's script resource system:

- `ScriptMod` metadata emitted by `script_mod!`;
- crate resource registration and deduplication;
- packaged dependency paths and WebAssembly HTTP URLs; and
- the platform-specific behavior of `res.file`.

Rust debug information, `file!()` source locations, panic metadata, and
toolchain path remapping are out of scope.

## Current Problem

`script_mod!` embeds `env!("CARGO_MANIFEST_DIR")` into every `ScriptMod`.
On a Windows build host this is an absolute path such as:

```text
C:\dev\makepad\widgets
```

Native builds legitimately use that directory for filesystem loading and hot
reload. WebAssembly instead compares it with resource paths to recover a
logical packaged path:

```text
C:\dev\makepad\widgets + resources/font.ttf
    -> makepad_widgets/resources/font.ttf
```

This makes WebAssembly depend on a filesystem that does not exist in the
browser, leaks build-host details into the artifact, and requires WebAssembly
to understand the build host's path syntax.

The resource reference already contains the required logical identity:

- `self:resources/font.ttf` obtains its crate from `module_path!()`; and
- `other_crate:resources/font.ttf` names the crate explicitly.

## Design

### Target-specific `ScriptMod` metadata

The proc macro will initialize `cargo_manifest_path` differently by target:

- native targets retain the current `env!("CARGO_MANIFEST_DIR")` value; and
- `wasm32` initializes it to an empty string without evaluating or embedding
  `CARGO_MANIFEST_DIR`.

The field remains in `ScriptMod` for a narrow, source-compatible patch.
Native live reload and file loading keep their current behavior.

`ScriptVm::add_script_mod` will not add empty manifest paths to
`crate_manifests`. Consequently the WebAssembly map contains no
build-host-derived directories.

### Logical WebAssembly crate resources

WebAssembly crate-resource resolution will no longer consult
`crate_manifests` or compare absolute path prefixes.

For `self:path`, the resolver takes the first component of the current
`ScriptMod.module_path` as the crate name. For `crate:path`, it uses the
explicit crate component. Both names retain the existing `-` to `_`
normalization.

The resource-relative portion is normalized by the existing
`normalize_dependency_file_path` helper. It accepts both separators, removes
`.` components, resolves `..`, and returns `None` if traversal would escape
the crate root.

The resolver produces:

```text
logical key:    crate://makepad_widgets/resources/font.ttf
dependency:     makepad_widgets/resources/font.ttf
web URL:        /makepad_widgets/resources/font.ttf
```

The logical key occupies the existing internal `abs_path` slot on WebAssembly
so resource-handle deduplication remains unchanged. It is a URI-like resource
identity, not a host path. Native builds continue storing actual absolute
paths in that slot.

Existing base-path and small-font URL transformations remain downstream of
the dependency path and are unchanged.

### `res.file` behavior

`res.file` remains an absolute filesystem escape hatch on native targets.

On WebAssembly it returns a script error:

```text
res.file is unavailable on wasm; use res.crate or res.http_resource
```

WebAssembly cannot open a host file, and there are no repository callers that
depend on its current manifest-prefix promotion behavior. Packaged assets use
`res.crate`; explicit network resources use `res.http_resource`; in-memory
bytes use `res.binary_resource`.

### Removed WebAssembly behavior

The following WebAssembly-only machinery becomes unnecessary:

- manifest-prefix candidate construction;
- longest absolute manifest-prefix selection;
- host-path separator canonicalization; and
- promotion of `res.file` absolute paths into packaged dependencies.

Native manifest maps and native absolute-path loading remain.

## Data Flow

For `self:resources/font.ttf` inside `makepad_widgets`:

1. `script_mod!` records `module_path` but embeds no manifest directory in
   WebAssembly.
2. `res.crate` parses `self` and the relative resource path.
3. The current script body provides `makepad_widgets` through `module_path`.
4. The relative path is normalized within the crate root.
5. The resource registry deduplicates by
   `crate://makepad_widgets/resources/font.ttf`.
6. Packaged dependency lookup uses
   `makepad_widgets/resources/font.ttf`.
7. If not preloaded, the WebAssembly loader fetches the corresponding URL.

No step constructs or compares a host filesystem path.

## Error Handling

Resolution returns a script error when:

- a crate resource lacks the `crate:path` shape;
- `self` cannot be associated with a script module;
- the explicit crate name is empty;
- the resource-relative path traverses above its crate root; or
- `res.file` is called on WebAssembly.

Missing dependency data and HTTP failures continue using the existing
resource error states.

## Compatibility

- Native resource loading and live reload are unchanged.
- Existing `res.crate` resource URLs retain their current
  `<crate>/<relative-path>` format.
- The WebAssembly resource registry's internal deduplication key changes from
  a build-host absolute path to a `crate://` identity.
- `get_resource_abs_path` has no repository callers. On WebAssembly it may
  return the logical `crate://` identity; native behavior remains unchanged.
- `res.file` becomes explicitly unsupported on WebAssembly. The repository
  contains no callers.

## Testing

Host-runnable unit tests will cover:

- self-crate logical identity;
- explicit cross-crate logical identity;
- stable dependency and URL construction;
- slash, backslash, and mixed-separator relative paths;
- `.` and valid `..` normalization;
- traversal above the crate root;
- identical logical keys for equivalent paths; and
- rejection of empty crate names.

Target-specific checks will cover:

- the platform crate compiling for `wasm32-unknown-unknown`;
- generated `ScriptMod.cargo_manifest_path` being empty on WebAssembly;
- native manifest-path registration remaining unchanged; and
- `res.file` producing the WebAssembly-specific guidance error.

The focused script-resource tests must pass. The repository's existing
`perf_monitor.rs` doctest and strict-Clippy baseline failures are recorded
separately and are not part of this change.

## Upstream Patch Shape

The upstream branch will be rebuilt from the latest `upstream/dev`. The
existing separator-normalizer commit will be replaced, not layered beneath
this design.

The submitted patch will contain the target-specific metadata change,
logical WebAssembly resolution, the explicit `res.file` error, and focused
regression coverage. It will not include the earlier general-purpose drive
or UNC path parser.
