#!/data/data/com.termux/files/usr/bin/bash
set -Eeuo pipefail

ROOT="$HOME/rustblend"
BUILD="$ROOT/build/apk"
STAGE="$BUILD/stage"

source "$HOME/.rustblend-env"

SDK="$ANDROID_HOME"
NDK="$ANDROID_NDK_HOME"

PLATFORM="$SDK/platforms/android-35/android.jar"
NATIVE_LIB="$ROOT/target/release/libmain.so"
NM="$NDK/toolchains/llvm/prebuilt/android/bin/llvm-nm"

UNSIGNED="$BUILD/rustblend-unsigned.apk"
ALIGNED="$BUILD/rustblend-aligned.apk"
SIGNED="$BUILD/rustblend-debug.apk"

KEYSTORE="$HOME/.android/rustblend-debug.keystore"
KEY_ALIAS="rustblend-debug"
KEY_PASSWORD="rustblend-debug"

AAPT2="$(command -v aapt2)"
ZIPALIGN="$(command -v zipalign)"
APKSIGNER="$(command -v apksigner)"
SEVEN_ZIP="$(command -v 7zz)"
KEYTOOL="$(command -v keytool)"

mkdir -p "$BUILD" "$STAGE/lib/arm64-v8a"
mkdir -p "$(dirname "$KEYSTORE")"

echo "=== RUSTBLEND APK DERLEYICISI ==="

# Önce Rust motorunu yeniden derle.
echo "=== RUST MOTORU DERLENIYOR ==="

env \
  -u CARGO_ENCODED_RUSTFLAGS \
  -u CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER \
  PATH="$PREFIX/bin:$PATH" \
  CC="$PREFIX/bin/clang" \
  CXX="$PREFIX/bin/clang++" \
  RUSTFLAGS="-C linker=$PREFIX/bin/clang" \
  CARGO_BUILD_JOBS=2 \
  cargo build --release --manifest-path "$ROOT/Cargo.toml"

# Gerekli dosyaları doğrula.
for FILE in \
    "$ROOT/AndroidManifest.xml" \
    "$PLATFORM" \
    "$NATIVE_LIB" \
    "$NM"
do
    if [ ! -f "$FILE" ]; then
        echo "HATA: Dosya bulunamadi: $FILE"
        exit 1
    fi
done

# Android giriş noktalarını doğrula.
echo "=== ANDROID GIRIS NOKTALARI ==="

"$NM" -D --defined-only "$NATIVE_LIB" > "$BUILD/symbols.txt"

grep -q 'ANativeActivity_onCreate' "$BUILD/symbols.txt"
grep -q 'android_main' "$BUILD/symbols.txt"

echo "Android giriş noktaları mevcut."

rm -f "$UNSIGNED" "$ALIGNED" "$SIGNED"

# Manifesti APK'ya bağla.
echo "=== MANIFEST BAGLANIYOR ==="

"$AAPT2" link \
    -o "$UNSIGNED" \
    --manifest "$ROOT/AndroidManifest.xml" \
    -I "$PLATFORM" \
    --min-sdk-version 23 \
    --target-sdk-version 30 \
    --version-code 1 \
    --version-name 0.1.0 \
    --debug-mode

# Rust kütüphanesini Android ARM64 klasörüne yerleştir.
echo "=== ARM64 KUTUPHANESI EKLENIYOR ==="

cp "$NATIVE_LIB" "$STAGE/lib/arm64-v8a/libmain.so"

(
    cd "$STAGE"
    "$SEVEN_ZIP" a -tzip -mx=0 \
        "$UNSIGNED" \
        "lib/arm64-v8a/libmain.so"
)

# 16 KB sayfa hizalaması.
echo "=== APK HIZALANIYOR ==="

"$ZIPALIGN" -P 16 -f -v 4 "$UNSIGNED" "$ALIGNED"

# Debug imza anahtarı.
if [ ! -f "$KEYSTORE" ]; then
    echo "=== DEBUG ANAHTARI OLUSTURULUYOR ==="

    "$KEYTOOL" -genkeypair \
        -keystore "$KEYSTORE" \
        -storetype PKCS12 \
        -alias "$KEY_ALIAS" \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -storepass "$KEY_PASSWORD" \
        -keypass "$KEY_PASSWORD" \
        -dname "CN=RUSTBLEND Debug,O=RUSTBLEND,C=TR"
fi

# APK'yı imzala.
echo "=== APK IMZALANIYOR ==="

"$APKSIGNER" sign \
    --ks "$KEYSTORE" \
    --ks-key-alias "$KEY_ALIAS" \
    --ks-pass "pass:$KEY_PASSWORD" \
    --key-pass "pass:$KEY_PASSWORD" \
    --out "$SIGNED" \
    "$ALIGNED"

# İmzayı ve hizalamayı denetle.
echo "=== IMZA KONTROLU ==="
"$APKSIGNER" verify --verbose "$SIGNED"

echo "=== HIZALAMA KONTROLU ==="
"$ZIPALIGN" -c -P 16 -v 4 "$SIGNED"

echo
echo "================================="
echo " RUSTBLEND APK BASARIYLA OLUSTU"
echo "================================="

ls -lh "$SIGNED"
echo "Dosya: $SIGNED"
