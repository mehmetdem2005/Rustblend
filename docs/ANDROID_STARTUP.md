# Android black-screen investigation — 0.5.1

## Evidence and limits

The 0.5.0 ARM64 APK built and signed successfully, but the user's Redmi Note 8
Pro / Android 11 still showed a completely black activity. This is a reproduced
user-visible failure, not a verified diagnosis of the device's driver.
No logcat from that phone is available. Do not claim the phone is fixed on the
basis of unit tests, compilation, or an x86 emulator.

## Confirmed code defects addressed

- A failed GPU initialization previously left `renderer=None` and waited forever.
  Setting NativeActivity's title does not paint a fallback/error screen.
- One combined Vulkan+GL instance could select Vulkan, fail later, and never
  retry a complete GLES renderer. Now try GLES first, then a separate Vulkan
  instance after recoverable initialization failure. Driver hangs/native crashes
  cannot be safely unwound into an automatic fallback.
- Default WebGPU limits were requested even though the editor uses only basic
  vertex/index/uniform buffers. Request GLES 3.0-compatible downlevel limits,
  respecting the adapter's texture resolution and buffer alignment limits.
- Validate the whole startup pipeline/resource scope. A broken UI pipeline is
  now an initialization failure, not a supposedly successful invisible editor.
- Report uncaptured GPU errors, initialization panics and exhausted frame retries.

## GPU-independent diagnostic display

`RustBlendActivity` extends NativeActivity and displays a non-focus-stealing
Android Dialog with its own window/surface. A child View in NativeActivity's
main window would not work: NativeActivity calls `takeSurface` for native rendering.
The main window explicitly requests RGBA8888 instead of NativeActivity's RGB565.

Rust atomically writes a bounded UTF-8 `files/startup-status.txt` (STARTING,
ERROR, READY); Java reads it while the activity is resumed. READY is written
only after first-frame submission/presentation with no observed device error.
A 15-second stalled stage offers a copyable device/stage report. This is not a
hardware timeout/cancellation mechanism. No project data is modified by reporting.
Old READY is removed on fresh Activity creation. Diagnostics remain available
for later GPU errors. The old app's ephemeral signing limitation still applies.

## Verification

- CPU tests include bounded UTF-8 status replacement and preservation of project data.
- Android-target compilation validates native integration.
- APK packaging compiles Java/D8, checks classes.dex and the launcher component,
  then validates signature, native architecture, alignment and package version.
- The opt-in APK workflow also builds an emulator-only x86_64 APK and launches
  Android 11 / SwiftShader, waits for READY, captures the viewport, exercises
  background/resume, then verifies the independent error dialog using a
  debug-only intent. Screenshots and logcat are retained as runtime evidence.
- Only the ARM64 APK is for the user's phone. Emulator success is not Mali proof.

Reference APIs: Android NativeActivity source (API 30), winit 0.30.13 lifecycle,
wgpu 30.0.1 device limits/error scopes; locked dependency sources were inspected.
