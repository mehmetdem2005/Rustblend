package com.rustblend.mobile;

import android.app.NativeActivity;
import android.app.Dialog;
import android.graphics.PixelFormat;
import android.view.WindowManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.graphics.Color;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.File;
import java.io.FileInputStream;
import java.nio.charset.StandardCharsets;

/** NativeActivity owns its main window's Surface, so ordinary child Views do
 * not render there. This dialog has an independent Android-rendered surface.
 */
public final class RustBlendActivity extends NativeActivity {
    private final Handler handler = new Handler(Looper.getMainLooper());
    private Dialog overlay;
    private TextView status;
    private Button copy;
    private File report;
    private String last = "";
    private String details = "Native motor henüz yanıt vermedi.";
    private long changedAt;
    private boolean forceError;

    @Override public void onCreate(Bundle saved) {
        report = new File(getFilesDir(), "startup-status.txt");
        // A previous process's READY must never hide a failed new startup.
        report.delete();
        super.onCreate(saved);
        getWindow().setFormat(PixelFormat.RGBA_8888);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setGravity(Gravity.CENTER_VERTICAL);
        int pad = (int)(24 * getResources().getDisplayMetrics().density);
        content.setPadding(pad, pad, pad, pad);
        content.setBackgroundColor(Color.rgb(21, 23, 27));
        TextView title = new TextView(this);
        title.setText("RustBlend 0.5.1"); title.setTextSize(26);
        title.setTextColor(Color.rgb(255, 124, 35));
        content.addView(title);
        status = new TextView(this);
        status.setText("3B çalışma alanı başlatılıyor…");
        status.setTextSize(17); status.setTextColor(Color.WHITE);
        status.setPadding(0, pad / 2, 0, pad / 2);
        content.addView(status);
        copy = new Button(this); copy.setText("Hata bilgisini kopyala");
        copy.setVisibility(View.GONE);
        copy.setOnClickListener(v -> {
            ClipboardManager clipboard = (ClipboardManager)getSystemService(CLIPBOARD_SERVICE);
            clipboard.setPrimaryClip(ClipData.newPlainText("RustBlend açılış raporu", details));
            copy.setText("Kopyalandı");
        });
        content.addView(copy);
        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true); scroll.addView(content);
        overlay = new Dialog(this);
        overlay.setContentView(scroll);
        overlay.setCancelable(false);
        overlay.getWindow().addFlags(WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE);
        overlay.getWindow().clearFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
        changedAt = SystemClock.elapsedRealtime();
        forceError = getIntent().getBooleanExtra("rustblend_test_error", false)
            && (getApplicationInfo().flags & android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE) != 0;
    }

    private String readReport() {
        if (forceError) return "ERROR\nTanılama ekranı testi (gerçek GPU hatası değil).";
        try (FileInputStream input = new FileInputStream(report)) {
            byte[] buffer = new byte[16384];
            int count = input.read(buffer);
            return count > 0 ? new String(buffer, 0, count, StandardCharsets.UTF_8) : "";
        } catch (Exception unavailable) { return ""; }
    }

    private final Runnable poll = new Runnable() {
        @Override public void run() {
            String current = readReport();
            long now = SystemClock.elapsedRealtime();
            if (!current.equals(last)) {
                last = current; changedAt = now;
                copy.setText("Hata bilgisini kopyala");
            }
            boolean ready = current.startsWith("READY\n");
            boolean failed = current.startsWith("ERROR\n");
            boolean stalled = !ready && now - changedAt >= 15000;
            String message = current.contains("\n") ? current.substring(current.indexOf('\n') + 1) : "Native motor bekleniyor";
            details = "RustBlend 0.5.1\n" + android.os.Build.MANUFACTURER + " " + android.os.Build.MODEL
                + " / Android " + android.os.Build.VERSION.RELEASE + "\n" + current;
            if (ready) {
                if (overlay.isShowing()) overlay.dismiss();
            } else if (!overlay.isShowing() && !isFinishing()) {
                overlay.show();
                overlay.getWindow().setLayout(ViewGroup.LayoutParams.MATCH_PARENT,
                    (int)(getResources().getDisplayMetrics().heightPixels * 0.8f));
            }
            if (!ready) {
                status.setText((failed ? "Grafik alanı açılamadı.\n\n" : stalled ? "Açılış beklenenden uzun sürdü.\n\n" : "3B çalışma alanı başlatılıyor…\n\n")
                    + message + ((failed || stalled) ? "\n\nProjelerin silinmedi. Hata bilgisini kopyalayıp paylaşabilirsin." : ""));
                copy.setVisibility(failed || stalled ? View.VISIBLE : View.GONE);
            }
            handler.postDelayed(this, ready ? 1000 : 300);
        }
    };
    @Override public void onResume() { super.onResume(); handler.removeCallbacks(poll); handler.post(poll); }
    @Override public void onPause() { handler.removeCallbacks(poll); overlay.dismiss(); super.onPause(); }
    @Override public void onDestroy() { handler.removeCallbacks(poll); overlay.dismiss(); super.onDestroy(); }
}
