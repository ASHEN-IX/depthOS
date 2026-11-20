# Phase 4.4 Project Context: Workspace Root and Safe Git Inspection

In DepthOS, a Workspace can be bound to a local filesystem directory representing a project or repository root.

## Path Validation Rules

Before attaching a directory to a Workspace record, the backend validates:

1. **Absolute Paths**: The path must be absolute (starting with `/`). Relative paths are rejected with an explicit error.
2. **Existence & Directory Type**: The target path must exist on the local filesystem and be a directory.
3. **Symlink Prohibition**: The directory path must not be a symbolic link or traverse symlinks that could escape containment.
4. **Permissions**: The directory must be readable by the executing user process.

## Safe, Non-Interactive Git Metadata Inspection

To give users immediate context without compromising host security:

- DepthOS executes `inspect_project_directory(path)` in Rust (`depthos-core`).
- It probes for `.git/HEAD` and `.git/refs/heads/` using safe, direct file inspection or strictly non-interactive subprocess calls:
  `git --no-optional-locks -C <path> rev-parse --abbrev-ref HEAD`
  `git --no-optional-locks -C <path> rev-parse --short HEAD`
- **Security Boundaries**:
  - Does **not** execute user-configured hooks (`core.hooksPath` ignored).
  - Does **not** fetch from remotes or open network sockets.
  - Does **not** modify repository files or lockfiles.
  - Does **not** run untrusted shell scripts.

## UI Presentation: Cyber-Tech Git Badge

Inside `ProjectRootPicker.qml`:
- As the user types or browses a directory, live validation occurs.
- If a valid Git repository is detected, an inline badge appears:
  `[ GIT: codex/phase-4-4 @ 7861b75 ✓ ]`
- If an invalid path or non-directory is entered, an error label provides clear feedback without blocking navigation.
