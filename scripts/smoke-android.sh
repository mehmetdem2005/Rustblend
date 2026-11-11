#!/usr/bin/env bash
# Actual Android launch: require a presented frame, exercise pause/resume, and
# verify the independently rendered error panel. Does not claim Mali coverage.
set -Eeuo pipefail
APK="${1:-build/android-apk/RUSTBLEND.apk}"
PACKAGE=com.rustblend.mobile
ACTIVITY="$PACKAGE/.RustBlendActivity"
mkdir -p build/smoke
trap 'adb logcat -d > build/smoke/logcat.txt || true' EXIT
adb install -r "$APK"
adb shell settings put system accelerometer_rotation 0
adb shell settings put system user_rotation 1
adb shell am start -W -n "$ACTIVITY"
wait_ready() {
    for attempt in $(seq 1 60); do
        report="$(adb shell run-as "$PACKAGE" cat files/startup-status.txt 2>/dev/null | tr -d '\r' || true)"
        if [[ "$report" == READY* ]]; then printf '%s\n' "$report"; return 0; fi
        if [[ "$report" == ERROR* ]]; then printf '%s\n' "$report" >&2; return 1; fi
        sleep 1
    done
    echo "No first frame within 60 seconds: $report" >&2; return 1
}
wait_ready
sleep 2
adb exec-out screencap -p > build/smoke/viewport.png
adb shell input keyevent KEYCODE_HOME
sleep 2
adb shell am start -W -n "$ACTIVITY"
wait_ready
sleep 2
adb exec-out screencap -p > build/smoke/resumed.png
# The emulator is disposable. Force-stop only its test installation.
adb shell am force-stop "$PACKAGE"
adb shell am start -W -n "$ACTIVITY" --ez rustblend_test_error true
sleep 4
adb shell uiautomator dump /sdcard/rustblend-ui.xml
adb pull /sdcard/rustblend-ui.xml build/smoke/error-ui.xml
python3 - <<'PY'
from pathlib import Path
text=Path('build/smoke/error-ui.xml').read_text()
assert 'Hata bilgisini kopyala' in text, text
assert 'Grafik alanı açılamadı.' in text, text
PY
adb exec-out screencap -p > build/smoke/error-panel.png
