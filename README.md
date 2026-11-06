# RUSTBLEND

RUSTBLEND is a native Android 3D modeling application written in Rust.

The project aims to bring a capable Blender-inspired 3D modeling
experience to Android devices.

## Project goals

- Native Android application.
- Rust-based architecture.
- Mobile-friendly 3D viewport.
- 3D mesh creation and editing.
- Camera navigation and object selection.
- GLB / GLTF import and export.
- Portrait and landscape support.
- Independent APK installation.
- Development and Android APK builds: GitHub Actions (cloud); Termux is not required.

## Target device

Initial development and testing target:

- Redmi Note 8 Pro
- Android 11
- ARM64
- Mali-G76 MC4

The project should remain portable and should not be limited
to this specific device.

## Development rules

### ONE BRANCH ONLY

This project MUST always be developed on one branch:

`main`

The following are strictly prohibited:

- Creating additional branches.
- Feature branches.
- Development branches.
- Experimental branches.
- Parallel development branches.
- Branch-based workflows.

All changes must be committed directly to `main`.

All contributors and AI assistants must follow this rule.
Do not create another branch under any circumstances.

### Preserve the existing project

- Continue developing the existing codebase.
- Do not create replacement projects unnecessarily.
- Preserve working features.
- Avoid destructive changes.
- Keep existing working architecture unless a justified
  improvement requires changing it.
- Test changes before building the APK.

### Development workflow

1. Inspect the existing code.
2. Understand the current architecture.
3. Make controlled changes directly on `main`.
4. Let GitHub Actions run `cargo test --locked`.
5. Let the GitHub runner cross-compile the Android ARM64 library.
6. Build, sign, and verify the APK in GitHub Actions.
7. Download the APK artifact from the successful workflow run.

### Security

Never commit:

- API tokens.
- Passwords.
- Private keys.
- Signing keystores.
- Personal credentials.
- Local environment files.

## Tests and Android APK builds

Development builds and automated checks run on GitHub Actions;
a local Termux environment is not required.

Workflow: [Android Build](.github/workflows/android-build.yml)

On every push to `main`, GitHub Actions:
1. Runs the Rust unit tests with `cargo test --locked`.
2. Cross-compiles the Android ARM64 native library using the Android NDK.
3. Packages, signs, and verifies the APK.
4. Publishes the installable APK as a workflow artifact for 14 days.

Open the repository's **Actions** tab, select the latest successful
"RUSTBLEND Android Build" run, and download its APK artifact.

The current cloud workflow generates a fresh temporary signing key on
each run. Therefore, each APK must be installed as a clean install after
removing an older RUSTBLEND APK signed with a different key. To support
in-place app updates, configure a persistent private keystore through
GitHub Actions secrets; never commit that keystore to this public repo.

The old Termux-only `build_apk.sh` is deprecated and is not used
by the GitHub Actions build. The active cloud build script is
`scripts/build-android-apk.sh`.

## Current workspace milestone (0.4.x)

The 0.4.0 device build was reported to close immediately on launch. Version 0.4.1 adds wgpu error logging, validation-scoped UI pipeline creation with a 3D-only fallback, and a Rust initialization-panic fallback. The device result is not yet verified.



The Android viewport now includes a touch-first modeling workspace:

- Add, duplicate, and delete cube objects.
- Select scene objects from the Outliner panel.
- Drag the viewport in Orbit, Move, Rotate, or Scale mode.
- Toggle Edit Preview to display mesh edges and vertex markers.
- GPU scene buffers are rebuilt when scene objects or transforms change.
- The compact toolbar, Outliner, and status strip are rendered by the native GPU renderer.

The Edit Preview currently visualizes mesh topology; direct vertex/edge/face deformation, undo/redo, and project file import/export remain future work.

## Development roadmap

See [PLAN.md](PLAN.md) for the current technical status, stabilization checklist, development phases, and acceptance criteria. Read it before continuing work in a new session.

## Repository policy

This is a public repository.

The single-branch `main` policy is mandatory for all future
development and contributions.
