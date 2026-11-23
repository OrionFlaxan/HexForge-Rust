package com.orionflaxan.hexforge;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Insets;
import android.os.Build;
import android.os.Bundle;
import android.system.Os;
import android.util.Log;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowManager.LayoutParams;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.LinearLayout;

import quad_native.QuadNative;

class QuadSurface
    extends SurfaceView
    implements View.OnTouchListener, View.OnKeyListener, SurfaceHolder.Callback {

    public QuadSurface(Context context) {
        super(context);
        getHolder().addCallback(this);
        setFocusable(true);
        setFocusableInTouchMode(true);
        requestFocus();
        setOnTouchListener(this);
        setOnKeyListener(this);
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceCreated(surface);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceDestroyed(surface);
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceChanged(surface, width, height);
    }

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        int pointerCount = event.getPointerCount();
        int action = event.getActionMasked();

        switch (action) {
            case MotionEvent.ACTION_MOVE: {
                for (int i = 0; i < pointerCount; i++) {
                    QuadNative.surfaceOnTouch(event.getPointerId(i), 0, event.getX(i), event.getY(i));
                }
                break;
            }
            case MotionEvent.ACTION_UP: {
                QuadNative.surfaceOnTouch(event.getPointerId(0), 1, event.getX(0), event.getY(0));
                break;
            }
            case MotionEvent.ACTION_DOWN: {
                QuadNative.surfaceOnTouch(event.getPointerId(0), 2, event.getX(0), event.getY(0));
                break;
            }
            case MotionEvent.ACTION_POINTER_UP: {
                int idx = event.getActionIndex();
                QuadNative.surfaceOnTouch(event.getPointerId(idx), 1, event.getX(idx), event.getY(idx));
                break;
            }
            case MotionEvent.ACTION_POINTER_DOWN: {
                int idx = event.getActionIndex();
                QuadNative.surfaceOnTouch(event.getPointerId(idx), 2, event.getX(idx), event.getY(idx));
                break;
            }
            case MotionEvent.ACTION_CANCEL: {
                for (int i = 0; i < pointerCount; i++) {
                    QuadNative.surfaceOnTouch(event.getPointerId(i), 3, event.getX(i), event.getY(i));
                }
                break;
            }
            default:
                break;
        }
        return true;
    }

    @SuppressWarnings("deprecation")
    @Override
    public boolean onKey(View v, int keyCode, KeyEvent event) {
        if (event.getAction() == KeyEvent.ACTION_DOWN && keyCode != 0) {
            QuadNative.surfaceOnKeyDown(keyCode);
        }
        if (event.getAction() == KeyEvent.ACTION_UP && keyCode != 0) {
            QuadNative.surfaceOnKeyUp(keyCode);
        }
        if (event.getAction() == KeyEvent.ACTION_UP || event.getAction() == KeyEvent.ACTION_MULTIPLE) {
            int character = event.getUnicodeChar();
            if (character == 0) {
                String characters = event.getCharacters();
                if (characters != null && !characters.isEmpty()) {
                    character = characters.charAt(0);
                }
            }
            if (character != 0) {
                QuadNative.surfaceOnCharacter(character);
            }
        }
        return true;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
        InputConnection connection = super.onCreateInputConnection(outAttrs);
        outAttrs.imeOptions |= EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return connection;
    }

    public Surface getNativeSurface() {
        return getHolder().getSurface();
    }
}

class ResizingLayout extends LinearLayout implements View.OnApplyWindowInsetsListener {
    public ResizingLayout(MainActivity activity) {
        super(activity);
        setBackgroundColor(Color.BLACK);
        setOnApplyWindowInsetsListener(this);
    }

    @Override
    public WindowInsets onApplyWindowInsets(View v, WindowInsets insets) {
        if (Build.VERSION.SDK_INT >= 30) {
            Insets imeInsets = insets.getInsets(WindowInsets.Type.ime());
            Insets sysInsets = insets.getInsets(WindowInsets.Type.systemBars());
            int bottomPadding = imeInsets.bottom > 0 ? imeInsets.bottom : sysInsets.bottom;
            v.setPadding(sysInsets.left, sysInsets.top, sysInsets.right, bottomPadding);
        }
        return insets;
    }
}

public class MainActivity extends Activity {
    private QuadSurface view;

    static {
        System.loadLibrary("hexforge_app");
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        try {
            Os.setenv("HEXFORGE_SAVE_DIR", getFilesDir().getAbsolutePath(), true);
            float density = getResources().getDisplayMetrics().density;
            Os.setenv("HEXFORGE_DPI_SCALE", String.valueOf(density), true);
        } catch (Throwable t) {
            Log.w("HexForge", "Failed to set HexForge environment variables", t);
        }

        this.requestWindowFeature(Window.FEATURE_NO_TITLE);
        setFullScreen(true);

        view = new QuadSurface(this);
        ResizingLayout layout = new ResizingLayout(this);
        layout.addView(view);
        setContentView(layout);

        QuadNative.activityOnCreate(this);
    }

    @Override
    protected void onResume() {
        super.onResume();
        setFullScreen(true);
        QuadNative.activityOnResume();
    }

    @Override
    protected void onPause() {
        super.onPause();
        QuadNative.activityOnPause();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        QuadNative.activityOnDestroy();
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
    }

    @SuppressWarnings("deprecation")
    public void setFullScreen(final boolean fullscreen) {
        runOnUiThread(() -> {
            View decorView = getWindow().getDecorView();
            if (fullscreen) {
                getWindow().setFlags(LayoutParams.FLAG_LAYOUT_NO_LIMITS, LayoutParams.FLAG_LAYOUT_NO_LIMITS);
                if (Build.VERSION.SDK_INT >= 28) {
                    getWindow().getAttributes().layoutInDisplayCutoutMode =
                        LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
                }
                if (Build.VERSION.SDK_INT >= 30) {
                    getWindow().setDecorFitsSystemWindows(false);
                } else {
                    int uiOptions = View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
                    decorView.setSystemUiVisibility(uiOptions);
                }
            } else {
                if (Build.VERSION.SDK_INT >= 30) {
                    getWindow().setDecorFitsSystemWindows(true);
                } else {
                    decorView.setSystemUiVisibility(0);
                }
            }
        });
    }

    public void showKeyboard(final boolean show) {
        runOnUiThread(() -> {
            InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
            if (imm != null) {
                if (show) {
                    imm.showSoftInput(view, 0);
                } else {
                    imm.hideSoftInputFromWindow(view.getWindowToken(), 0);
                }
            }
        });
    }

    public String getClipboardText() {
        ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboard == null || !clipboard.hasPrimaryClip()) return null;
        ClipData primaryClip = clipboard.getPrimaryClip();
        if (primaryClip == null || primaryClip.getItemCount() < 1) return null;
        CharSequence clipData = primaryClip.getItemAt(0).getText();
        return clipData != null ? clipData.toString() : null;
    }

    public void setClipboardText(String text) {
        ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboard != null) {
            clipboard.setPrimaryClip(ClipData.newPlainText("HexForge", text));
        }
    }
}
