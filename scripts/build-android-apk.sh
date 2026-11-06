#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="${GITHUB_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
BUILD_DIR="$ROOT/build/android-apk"
STAGE="$BUILD_DIR/stage"
TARGET="aarch64-linux-android"
API_LEVEL="23"
BUILD_TOOLS_VERSION="35.0.0"
NDK_VERSION="29.0.14206865"

SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$SDK_ROOT" || ! -d "$SDK_ROOT" ]]; then
    echo "ERROR: Android SDK is not configured (ANDROID_HOME)." >&2
    exit 1
fi

NDK_ROOT="${ANDROID_NDK_HOME:-$SDK_ROOT/ndk/$NDK_VERSION}"
HOST_TAG="linux-x86_64"
TOOLCHAIN="$NDK_ROOT/toolchains/llvm/prebuilt/$HOST_TAG"
BUILD_TOOLS="$SDK_ROOT/build-tools/$BUILD_TOOLS_VERSION"
ANDROID_JAR="$SDK_ROOT/platforms/android-35/android.jar"

for path in \
    "$TOOLCHAIN/bin/aarch64-linux-android${API_LEVEL}-clang" \
    "$TOOLCHAIN/bin/llvm-ar" \
    "$TOOLCHAIN/bin/llvm-nm" \
    "$BUILD_TOOLS/aapt2" \
    "$BUILD_TOOLS/zipalign" \
    "$BUILD_TOOLS/apksigner" \
    "$ANDROID_JAR"
do
    if [[ ! -x "$path" && ! -f "$path" ]]; then
        echo "ERROR: Required Android build tool not found: $path" >&2
        exit 1
    fi
done

export ANDROID_NDK_HOME="$NDK_ROOT"
export NDK_HOME="$NDK_ROOT"
export CC_aarch64_linux_android="$TOOLCHAIN/bin/aarch64-linux-android${API_LEVEL}-clang"
export AR_aarch64_linux_android="$TOOLCHAIN/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/aarch64-linux-android${API_LEVEL}-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"

NATIVE_LIB="$ROOT/target/$TARGET/release/libmain.so"
UNSIGNED="$BUILD_DIR/rustblend-unsigned.apk"
ALIGNED="$BUILD_DIR/rustblend-aligned.apk"
SIGNED="$BUILD_DIR/RUSTBLEND.apk"
KEYSTORE="$BUILD_DIR/ci-debug.keystore"

mkdir -p "$BUILD_DIR" "$STAGE/lib/arm64-v8a"
rm -rf "$STAGE/lib/arm64-v8a/libmain.so"
rm -f "$UNSIGNED" "$ALIGNED" "$SIGNED"

echo "=== RUSTBLEND: Android ARM64 release build ==="
cargo build --locked --release --target "$TARGET" --manifest-path "$ROOT/Cargo.toml"

if [[ ! -s "$NATIVE_LIB" ]]; then
    echo "ERROR: Rust Android shared library was not produced: $NATIVE_LIB" >&2
    exit 1
fi

"$TOOLCHAIN/bin/llvm-nm" -D --defined-only "$NATIVE_LIB" > "$BUILD_DIR/native-symbols.txt"
grep -q 'ANativeActivity_onCreate' "$BUILD_DIR/native-symbols.txt"
grep -q 'android_main' "$BUILD_DIR/native-symbols.txt"
echo "Android NativeActivity entry points verified."

VERSION_CODE="$(sed -n 's/.*android:versionCode="\([^"]*\)".*/\1/p' "$ROOT/AndroidManifest.xml" | head -n 1)"
VERSION_NAME="$(sed -n 's/.*android:versionName="\([^"]*\)".*/\1/p' "$ROOT/AndroidManifest.xml" | head -n 1)"
if [[ -z "$VERSION_CODE" || -z "$VERSION_NAME" ]]; then
    echo "ERROR: Could not read versionCode/versionName from AndroidManifest.xml." >&2
    exit 1
fi

echo "=== Packaging version $VERSION_NAME ($VERSION_CODE) ==="
"$BUILD_TOOLS/aapt2" link \
    -o "$UNSIGNED" \
    --manifest "$ROOT/AndroidManifest.xml" \
    -I "$ANDROID_JAR" \
    --min-sdk-version 23 \
    --target-sdk-version 30 \
    --version-code "$VERSION_CODE" \
    --version-name "$VERSION_NAME" \
    --debug-mode

cp "$NATIVE_LIB" "$STAGE/lib/arm64-v8a/libmain.so"
(
    cd "$STAGE"
    zip -q -0 "$UNSIGNED" "lib/arm64-v8a/libmain.so"
)

"$BUILD_TOOLS/zipalign" -P 16 -f -v 4 "$UNSIGNED" "$ALIGNED"

# CI signing key is intentionally ephemeral. For this first device test,
# install after removing any locally signed development build.
keytool -genkeypair -noprompt \
    -keystore "$KEYSTORE" \
    -storetype PKCS12 \
    -alias rustblend-ci \
    -keyalg RSA \
    -keysize 2048 \
    -validity 10000 \
    -storepass android \
    -keypass android \
    -dname "CN=RUSTBLEND CI,O=RUSTBLEND,C=TR"

"$BUILD_TOOLS/apksigner" sign \
    --ks "$KEYSTORE" \
    --ks-key-alias rustblend-ci \
    --ks-pass pass:android \
    --key-pass pass:android \
    --out "$SIGNED" \
    "$ALIGNED"

"$BUILD_TOOLS/apksigner" verify --verbose "$SIGNED"
"$BUILD_TOOLS/zipalign" -c -P 16 -v 4 "$SIGNED"

echo "=== APK BUILD COMPLETE ==="
ls -lh "$SIGNED"
echo "APK: $SIGNED"
