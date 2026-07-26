# Resource Path Normalization Design

> Superseded by
> `2026-07-26-wasm-logical-resource-identity-design.md`. The replacement
> removes build-host manifest paths from WebAssembly resource identity rather
> than teaching WebAssembly to normalize host filesystem paths.

## Goal

Make crate-resource lookup work in WebAssembly binaries built on Windows
without allowing lexical path normalization to escape an absolute path's
root.

Windows build hosts embed backslash-separated `CARGO_MANIFEST_DIR` values in
the WebAssembly binary. On WebAssembly, `std::path` applies POSIX semantics, so
those backslashes are ordinary characters and manifest-prefix comparison
fails. Normalization must therefore recognize both separators independently
of the host platform.

## Scope

This change is limited to lexical normalization used by WebAssembly
crate-resource resolution. It will:

- accept `/` and `\` as separators;
- resolve `.` and `..` components;
- preserve relative, POSIX, Windows drive, and UNC anchors;
- emit `/`-separated normalized paths; and
- reject traversal above an anchor.

It will not access the filesystem, canonicalize symlinks, change resource URL
formatting, or alter longest-manifest-prefix selection.

## Design

`normalize_path_str` will parse the path into an immutable anchor and a
mutable component stack.

The supported anchors are:

- no anchor for relative paths;
- `/` for POSIX absolute paths;
- `C:/` for Windows drive-absolute paths, with any ASCII drive letter; and
- `//server/share/` for UNC paths.

Empty components and `.` are discarded. A normal component is pushed onto
the stack. A `..` pops one normal component; if none remains, normalization
returns `None` rather than removing or crossing the anchor.

The normalized result reconstructs the immutable anchor followed by the
remaining components using `/`. Drive-relative forms such as `C:foo` are not
promoted to drive-absolute paths; they retain relative-path behavior.
Incomplete UNC forms without both a server and share are rejected.

Examples:

| Input | Result |
|---|---|
| `a/b/../c` | `a/c` |
| `/a/../c` | `/c` |
| `C:\a\..\c` | `C:/c` |
| `\\server\share\a\..\c` | `//server/share/c` |
| `/../c` | `None` |
| `C:\..\c` | `None` |
| `\\server\share\..\c` | `None` |

## Integration

The existing WebAssembly `normalize_path` wrapper will continue converting
`Path` to a string, calling `normalize_path_str`, and converting the result
back to `PathBuf`. `resolve_dependency_path_from_manifests` will continue to
normalize the absolute resource path and all candidate manifest paths before
performing longest-prefix selection.

No public API changes are required.

## Error Handling

Normalization returns `None` for:

- `..` that would traverse above a relative or absolute anchor;
- a drive-absolute path that attempts to remove its drive root; and
- malformed or incomplete UNC paths.

Callers already treat `None` as an unresolvable resource path, so no new error
channel is introduced.

## Testing

Tests run on every host by compiling `normalize_path_str` under
`cfg(any(target_arch = "wasm32", test))`.

The focused test matrix covers:

- equivalent slash and backslash spellings;
- mixed-separator parent resolution;
- relative paths with valid and excessive `..`;
- POSIX roots with valid and above-root traversal;
- drive roots with valid and above-root traversal;
- UNC shares with valid and above-share traversal;
- malformed UNC inputs; and
- unchanged longest-manifest-prefix behavior.

Implementation follows red-green-refactor: add the drive-root and UNC
regressions first, verify that the current implementation fails for the
expected anchor-escape behavior, then make the smallest normalizer change and
rerun the focused and surrounding platform tests.

## Upstream Patch Shape

The upstream submission will contain one behavior-focused commit with the
normalizer, regression tests, and an explanation of why host `std::path`
semantics cannot normalize build-host paths embedded in WebAssembly. It
will remain independent of the other WAML fork commits.
