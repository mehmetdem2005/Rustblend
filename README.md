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
- Development environment: Termux on Android.

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
3. Make controlled changes.
4. Run `cargo test`.
5. Run `cargo build --release`.
6. Build and verify the Android APK.
7. Commit changes directly to `main`.

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

The cloud build currently uses a temporary CI signing key. The first
cloud-signed APK therefore cannot update an APK signed with the old
local debug key; uninstall the old test app before installing the
first cloud build. A persistent protected signing key can be configured
later through GitHub Actions secrets.

The legacy `build_apk.sh` is retained for reference and is not used
by the GitHub Actions build.

## Development roadmap

See [PLAN.md](PLAN.md) for the current technical status, stabilization checklist, development phases, and acceptance criteria. Read it before continuing work in a new session.

## Repository policy

This is a public repository.

The single-branch `main` policy is mandatory for all future
development and contributions.
