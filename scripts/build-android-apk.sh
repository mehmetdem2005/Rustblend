#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="${GITHUB_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
BUILD_DIR="$ROOT/build/android-apk"
STAGE="$BUILD_DIR/stage"
TARGET="${RUSTBLEND_TARGET:-aarch64-linux-android}"
case "$TARGET" in
    aarch64-linux-android) ABI="arm64-v8a"; ELF_MACHINE="AArch64" ;;
    x86_64-linux-android) ABI="x86_64"; ELF_MACHINE="Advanced Micro Devices X86-64" ;;
    *) echo "Unsupported Android target: $TARGET" >&2; exit 1 ;;
esac
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

REQUIRED_TOOLS=(
    "$TOOLCHAIN/bin/${TARGET}${API_LEVEL}-clang"
    "$TOOLCHAIN/bin/llvm-ar"
    "$TOOLCHAIN/bin/llvm-nm"
    "$TOOLCHAIN/bin/llvm-readelf"
    "$BUILD_TOOLS/d8"
    "$BUILD_TOOLS/aapt2"
    "$BUILD_TOOLS/aapt"
    "$BUILD_TOOLS/zipalign"
    "$BUILD_TOOLS/apksigner"
    "$ANDROID_JAR"
)

for path in "${REQUIRED_TOOLS[@]}"; do
    if [[ ! -x "$path" && ! -f "$path" ]]; then
        echo "ERROR: Required Android build tool not found: $path" >&2
        exit 1
    fi
done

command -v javac >/dev/null
command -v cargo >/dev/null
command -v zip >/dev/null
command -v unzip >/dev/null
command -v keytool >/dev/null

export ANDROID_NDK_HOME="$NDK_ROOT"
export NDK_HOME="$NDK_ROOT"
TARGET_UNDERSCORE="${TARGET//-/_}"
TARGET_UPPER="${TARGET_UNDERSCORE^^}"
export "CC_${TARGET_UNDERSCORE}=$TOOLCHAIN/bin/${TARGET}${API_LEVEL}-clang"
export "AR_${TARGET_UNDERSCORE}=$TOOLCHAIN/bin/llvm-ar"
export "CARGO_TARGET_${TARGET_UPPER}_LINKER=$TOOLCHAIN/bin/${TARGET}${API_LEVEL}-clang"
export "CARGO_TARGET_${TARGET_UPPER}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

NATIVE_LIB="$ROOT/target/$TARGET/release/libmain.so"
UNSIGNED="$BUILD_DIR/rustblend-unsigned.apk"
ALIGNED="$BUILD_DIR/rustblend-aligned.apk"
SIGNED="$BUILD_DIR/RUSTBLEND.apk"
KEYSTORE="$BUILD_DIR/ci-debug.keystore"

mkdir -p "$BUILD_DIR" "$STAGE/lib/$ABI"
rm -f "$STAGE/lib/$ABI/libmain.so" "$UNSIGNED" "$ALIGNED" "$SIGNED"

echo "=== RUSTBLEND: Android ARM64 release build ==="
cargo build --locked --release --target "$TARGET" --manifest-path "$ROOT/Cargo.toml"

if [[ ! -s "$NATIVE_LIB" ]]; then
    echo "ERROR: Rust Android shared library was not produced: $NATIVE_LIB" >&2
    exit 1
fi

echo "=== Native library architecture and Android entry points ==="
"$TOOLCHAIN/bin/llvm-readelf" -h "$NATIVE_LIB" > "$BUILD_DIR/native-header.txt"
grep -q "$ELF_MACHINE" "$BUILD_DIR/native-header.txt"
"$TOOLCHAIN/bin/llvm-nm" -D --defined-only "$NATIVE_LIB" > "$BUILD_DIR/native-symbols.txt"
grep -q "ANativeActivity_onCreate" "$BUILD_DIR/native-symbols.txt"
grep -q "android_main" "$BUILD_DIR/native-symbols.txt"

"$TOOLCHAIN/bin/llvm-readelf" -lW "$NATIVE_LIB" > "$BUILD_DIR/native-program-headers.txt"
mapfile -t LOAD_ALIGNMENTS < <(awk '$1 == "LOAD" { print $NF }' "$BUILD_DIR/native-program-headers.txt")
if [[ "${#LOAD_ALIGNMENTS[@]}" -eq 0 ]]; then
    echo "ERROR: No ELF LOAD segments found in native library." >&2
    exit 1
fi
for alignment in "${LOAD_ALIGNMENTS[@]}"; do
    if (( alignment < 0x4000 )); then
        echo "ERROR: Native ELF LOAD segment alignment is below 16 KB: $alignment" >&2
        exit 1
    fi
done
echo "Android ARM64 entry points and 16 KB ELF LOAD alignment verified."

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

echo "=== Compile GPU-independent Android startup diagnostics ==="
JAVA_CLASSES="$BUILD_DIR/java-classes"
DEX_DIR="$BUILD_DIR/dex"
mkdir -p "$JAVA_CLASSES" "$DEX_DIR"
javac -encoding UTF-8 -source 8 -target 8 -bootclasspath "$ANDROID_JAR" \
    -d "$JAVA_CLASSES" "$ROOT/android/java/com/rustblend/mobile/RustBlendActivity.java"
mapfile -t JAVA_INPUTS < <(find "$JAVA_CLASSES" -name '*.class' -type f)
"$BUILD_TOOLS/d8" --min-api "$API_LEVEL" --lib "$ANDROID_JAR" --output "$DEX_DIR" "${JAVA_INPUTS[@]}"
cp "$DEX_DIR/classes.dex" "$STAGE/classes.dex"
cp "$NATIVE_LIB" "$STAGE/lib/$ABI/libmain.so"
(
    cd "$STAGE"
    zip -q -0 "$UNSIGNED" "lib/$ABI/libmain.so" "classes.dex"
)

"$BUILD_TOOLS/zipalign" -P 16 -f -v 4 "$UNSIGNED" "$ALIGNED"

# The CI signing key is generated on the runner and is never committed.
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

echo "=== APK signature and alignment verification ==="
"$BUILD_TOOLS/apksigner" verify --verbose --print-certs "$SIGNED"
"$BUILD_TOOLS/zipalign" -c -P 16 -v 4 "$SIGNED"

echo "=== APK package metadata verification ==="
"$BUILD_TOOLS/aapt" dump badging "$SIGNED" > "$BUILD_DIR/apk-badging.txt"
grep -F "package: name='com.rustblend.mobile' versionCode='$VERSION_CODE' versionName='$VERSION_NAME'" "$BUILD_DIR/apk-badging.txt"
grep -F "sdkVersion:'23'" "$BUILD_DIR/apk-badging.txt"
grep -F "targetSdkVersion:'30'" "$BUILD_DIR/apk-badging.txt"
unzip -Z1 "$SIGNED" | grep -Fx "lib/$ABI/libmain.so" >/dev/null
unzip -Z1 "$SIGNED" | grep -Fx "classes.dex" >/dev/null
grep -F "launchable-activity: name='com.rustblend.mobile.RustBlendActivity'" "$BUILD_DIR/apk-badging.txt"

echo "=== APK BUILD COMPLETE ==="
ls -lh "$SIGNED"
echo "APK: $SIGNED"
