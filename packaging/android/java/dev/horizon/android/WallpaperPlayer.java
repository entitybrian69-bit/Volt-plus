package dev.horizon.android;

import android.graphics.SurfaceTexture;
import android.media.MediaPlayer;
import android.opengl.EGL14;
import android.opengl.GLES11Ext;
import android.opengl.GLES20;
import android.os.Handler;
import android.os.HandlerThread;
import android.view.Surface;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;

/** Decodes the original MP4 on a worker looper and exposes at most one RGBA frame.
 * The launcher uploads it into its own Vulkan texture; no transparent window is required. */
final class WallpaperPlayer {
    private static final WallpaperPlayer INSTANCE = new WallpaperPlayer();
    static WallpaperPlayer get() { return INSTANCE; }

    private final HandlerThread thread = new HandlerThread("Volt wallpaper decoder");
    private final Handler handler;
    private final Object lock = new Object();
    private ByteBuffer pixels;
    private boolean ready, reading;
    private int generation;
    private String status = "";
    private android.opengl.EGLDisplay display = EGL14.EGL_NO_DISPLAY;
    private android.opengl.EGLContext context = EGL14.EGL_NO_CONTEXT;
    private android.opengl.EGLSurface eglSurface = EGL14.EGL_NO_SURFACE;
    private SurfaceTexture surfaceTexture;
    private Surface surface;
    private MediaPlayer player;
    private int texture, program, width, height;
    private long lastFrameNanos;
    private static final long FRAME_INTERVAL_NANOS = 60_000_000L; // roughly 15 uploads/sec from 30 fps sources
    private final float[] transform = new float[16];
    private final FloatBuffer quad = floats(new float[] {
            -1,-1, 0,1,  1,-1, 1,1,  -1,1, 0,0,  1,1, 1,0 });

    private WallpaperPlayer() { thread.start(); handler = new Handler(thread.getLooper()); }
    private static FloatBuffer floats(float[] data) {
        FloatBuffer b = ByteBuffer.allocateDirect(data.length * 4).order(ByteOrder.nativeOrder()).asFloatBuffer();
        b.put(data).position(0); return b;
    }

    static int[] probeSize(String path) {
        android.media.MediaMetadataRetriever r = new android.media.MediaMetadataRetriever();
        try {
            r.setDataSource(path);
            String ws = r.extractMetadata(android.media.MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH);
            String hs = r.extractMetadata(android.media.MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT);
            String rot = r.extractMetadata(android.media.MediaMetadataRetriever.METADATA_KEY_VIDEO_ROTATION);
            int w = ws != null ? Integer.parseInt(ws) : 0;
            int h = hs != null ? Integer.parseInt(hs) : 0;
            int deg = rot != null ? Integer.parseInt(rot) : 0;
            if ((deg == 90 || deg == 270) && w > 0 && h > 0) {
                int tmp = w; w = h; h = tmp;
            }
            if (w > 0 && h > 0) return new int[] { w, h };
        } catch (Throwable ignored) {
        } finally {
            try { r.release(); } catch (Throwable ignored) {}
        }
        return new int[] { 1280, 720 };
    }

    static boolean extractThumbnail(String videoPath, String outJpgPath, int targetW, int targetH) {
        if (videoPath == null || outJpgPath == null || targetW <= 0 || targetH <= 0) return false;
        android.media.MediaMetadataRetriever r = new android.media.MediaMetadataRetriever();
        android.graphics.Bitmap raw = null;
        android.graphics.Bitmap scaled = null;
        try {
            r.setDataSource(videoPath);
            raw = r.getFrameAtTime(500_000L, android.media.MediaMetadataRetriever.OPTION_CLOSEST_SYNC);
            if (raw == null) {
                raw = r.getFrameAtTime(0L, android.media.MediaMetadataRetriever.OPTION_CLOSEST_SYNC);
            }
            if (raw == null) return false;
            int rw = raw.getWidth();
            int rh = raw.getHeight();
            if (rw <= 0 || rh <= 0) return false;
            // Center-crop scale to targetW x targetH without distortion.
            float scale = Math.max((float) targetW / rw, (float) targetH / rh);
            int sw = Math.max(targetW, Math.round(rw * scale));
            int sh = Math.max(targetH, Math.round(rh * scale));
            android.graphics.Bitmap resized = android.graphics.Bitmap.createScaledBitmap(raw, sw, sh, true);
            int ox = Math.max(0, (sw - targetW) / 2);
            int oy = Math.max(0, (sh - targetH) / 2);
            scaled = android.graphics.Bitmap.createBitmap(resized, ox, oy, targetW, targetH);
            if (resized != raw && resized != scaled) resized.recycle();
            java.io.File outFile = new java.io.File(outJpgPath);
            java.io.File parent = outFile.getParentFile();
            if (parent != null && !parent.exists()) parent.mkdirs();
            java.io.File tmp = new java.io.File(outJpgPath + ".part");
            try (java.io.FileOutputStream fos = new java.io.FileOutputStream(tmp)) {
                scaled.compress(android.graphics.Bitmap.CompressFormat.JPEG, 84, fos);
                fos.flush();
            }
            return tmp.renameTo(outFile);
        } catch (Throwable ignored) {
            return false;
        } finally {
            if (scaled != null) scaled.recycle();
            if (raw != null) raw.recycle();
            try { r.release(); } catch (Throwable ignored) {}
        }
    }

    void start(String path, int sourceWidth, int sourceHeight, int w, int h) {
        final int id;
        synchronized (lock) { id = ++generation; ready = false; status = "Preparing wallpaper..."; }
        handler.post(() -> {
            releaseOnThread();
            synchronized (lock) { if (id != generation) return; }
            try {
                if (w <= 0 || h <= 0 || (long) w * h * 4 > 40_000_000L) throw new IllegalArgumentException("Unsupported video size");
                int srcW = sourceWidth;
                int srcH = sourceHeight;
                if (srcW <= 0 || srcH <= 0) {
                    int[] probed = probeSize(path);
                    srcW = probed[0];
                    srcH = probed[1];
                }
                width = w; height = h; lastFrameNanos = 0;
                initGl();
                surfaceTexture = new SurfaceTexture(texture);
                // Decode the untouched source; GL samples to the physical display + overscan.
                surfaceTexture.setDefaultBufferSize(srcW, srcH);
                surface = new Surface(surfaceTexture);
                surfaceTexture.setOnFrameAvailableListener(st -> frame(id), handler);
                MediaPlayer m = new MediaPlayer();
                player = m;
                m.setDataSource(path);
                m.setSurface(surface);
                m.setVolume(0f, 0f);
                m.setLooping(true);
                m.setOnVideoSizeChangedListener((p, vw, vh) -> {
                    synchronized (lock) {
                        if (id == generation && surfaceTexture != null && vw > 0 && vh > 0) {
                            surfaceTexture.setDefaultBufferSize(vw, vh);
                        }
                    }
                });
                m.setOnPreparedListener(p -> {
                    synchronized (lock) { if (id != generation) return; status = ""; }
                    p.start();
                });
                m.setOnErrorListener((p, what, extra) -> {
                    synchronized (lock) { if (id == generation) status = "Wallpaper video playback failed (" + what + ")"; }
                    releaseOnThread();
                    return true;
                });
                m.prepareAsync();
            } catch (Throwable e) {
                synchronized (lock) { if (id == generation) status = "Wallpaper video unavailable: " + e.getClass().getSimpleName(); }
                releaseOnThread();
            }
        });
    }
    void stop() {
        synchronized (lock) { ++generation; ready = false; status = ""; }
        handler.post(this::releaseOnThread);
    }
    String status() { synchronized (lock) { return status; } }
    ByteBuffer acquire() {
        synchronized (lock) {
            if (!ready || reading || pixels == null) return null;
            reading = true;
            pixels.position(0);
            return pixels;
        }
    }
    void releaseFrame() { synchronized (lock) { reading = false; ready = false; } }

    private static int shader(int type, String source) {
        int s = GLES20.glCreateShader(type);
        GLES20.glShaderSource(s, source); GLES20.glCompileShader(s);
        int[] ok = new int[1]; GLES20.glGetShaderiv(s, GLES20.GL_COMPILE_STATUS, ok, 0);
        if (ok[0] == 0) throw new IllegalStateException("wallpaper shader: " + GLES20.glGetShaderInfoLog(s));
        return s;
    }
    private void initGl() {
        // EGL_DEFAULT_DISPLAY can also be in use by the game's OpenGL backend.
        // Initialize it once and never terminate that shared display on stop.
        if (display == EGL14.EGL_NO_DISPLAY) {
            display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY);
            int[] version = new int[2];
            if (display == EGL14.EGL_NO_DISPLAY || !EGL14.eglInitialize(display, version, 0, version, 1)) {
                display = EGL14.EGL_NO_DISPLAY;
                throw new IllegalStateException("EGL init");
            }
        }
        int[] attrs = { EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
                EGL14.EGL_SURFACE_TYPE, EGL14.EGL_PBUFFER_BIT,
                EGL14.EGL_RED_SIZE, 8, EGL14.EGL_GREEN_SIZE, 8,
                EGL14.EGL_BLUE_SIZE, 8, EGL14.EGL_ALPHA_SIZE, 8, EGL14.EGL_NONE };
        android.opengl.EGLConfig[] configs = new android.opengl.EGLConfig[1]; int[] count = new int[1];
        if (!EGL14.eglChooseConfig(display, attrs, 0, configs, 0, 1, count, 0) || count[0] == 0) throw new IllegalStateException("EGL config");
        context = EGL14.eglCreateContext(display, configs[0], EGL14.EGL_NO_CONTEXT,
                new int[] { EGL14.EGL_CONTEXT_CLIENT_VERSION, 2, EGL14.EGL_NONE }, 0);
        eglSurface = EGL14.eglCreatePbufferSurface(display, configs[0],
                new int[] { EGL14.EGL_WIDTH, width, EGL14.EGL_HEIGHT, height, EGL14.EGL_NONE }, 0);
        if (context == EGL14.EGL_NO_CONTEXT || eglSurface == EGL14.EGL_NO_SURFACE || !EGL14.eglMakeCurrent(display, eglSurface, eglSurface, context)) throw new IllegalStateException("EGL surface");
        int[] ids = new int[1]; GLES20.glGenTextures(1, ids, 0); texture = ids[0];
        GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, texture);
        GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR);
        GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_LINEAR);
        GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE);
        GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE);
        int vs = shader(GLES20.GL_VERTEX_SHADER, "attribute vec2 p; attribute vec2 uv; uniform mat4 texMatrix; varying vec2 v; void main(){gl_Position=vec4(p,0.,1.); v=(texMatrix*vec4(uv,0.,1.)).xy;}");
        int fs = shader(GLES20.GL_FRAGMENT_SHADER, "#extension GL_OES_EGL_image_external : require\nprecision mediump float; varying vec2 v; uniform samplerExternalOES tex; void main(){gl_FragColor=texture2D(tex,v);}");
        program = GLES20.glCreateProgram(); GLES20.glAttachShader(program, vs); GLES20.glAttachShader(program, fs); GLES20.glLinkProgram(program);
        GLES20.glDeleteShader(vs); GLES20.glDeleteShader(fs);
        int[] ok = new int[1]; GLES20.glGetProgramiv(program, GLES20.GL_LINK_STATUS, ok, 0);
        if (ok[0] == 0) throw new IllegalStateException("wallpaper shader link");
        synchronized (lock) { pixels = ByteBuffer.allocateDirect(width * height * 4).order(ByteOrder.nativeOrder()); ready = false; }
    }
    private void frame(int id) {
        synchronized (lock) { if (id != generation || surfaceTexture == null) return; }
        try {
            surfaceTexture.updateTexImage();
            synchronized (lock) { if (ready || reading || id != generation) return; }
            long now = android.os.SystemClock.elapsedRealtimeNanos();
            if (lastFrameNanos != 0 && now - lastFrameNanos < FRAME_INTERVAL_NANOS) return;
            lastFrameNanos = now;
            surfaceTexture.getTransformMatrix(transform);
            GLES20.glViewport(0, 0, width, height);
            GLES20.glUseProgram(program);
            int p = GLES20.glGetAttribLocation(program, "p"), uv = GLES20.glGetAttribLocation(program, "uv");
            quad.position(0); GLES20.glVertexAttribPointer(p, 2, GLES20.GL_FLOAT, false, 16, quad);
            quad.position(2); GLES20.glVertexAttribPointer(uv, 2, GLES20.GL_FLOAT, false, 16, quad);
            GLES20.glEnableVertexAttribArray(p); GLES20.glEnableVertexAttribArray(uv);
            GLES20.glUniformMatrix4fv(GLES20.glGetUniformLocation(program, "texMatrix"), 1, false, transform, 0);
            GLES20.glActiveTexture(GLES20.GL_TEXTURE0); GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, texture);
            GLES20.glUniform1i(GLES20.glGetUniformLocation(program, "tex"), 0);
            GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4);
            pixels.position(0);
            GLES20.glReadPixels(0, 0, width, height, GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, pixels);
            if (GLES20.glGetError() != GLES20.GL_NO_ERROR) throw new IllegalStateException("Frame readback failed");
            synchronized (lock) { if (id == generation) ready = true; }
        } catch (Throwable e) {
            synchronized (lock) { if (id == generation) status = "Wallpaper frame failed: " + e.getClass().getSimpleName(); }
            releaseOnThread();
        }
    }
    private void releaseOnThread() {
        if (player != null) { try { player.release(); } catch (Throwable ignored) {} player = null; }
        if (surface != null) { surface.release(); surface = null; }
        if (surfaceTexture != null) { surfaceTexture.release(); surfaceTexture = null; }
        if (display != EGL14.EGL_NO_DISPLAY && context != EGL14.EGL_NO_CONTEXT) {
            if (program != 0) GLES20.glDeleteProgram(program);
            if (texture != 0) GLES20.glDeleteTextures(1, new int[] { texture }, 0);
            EGL14.eglMakeCurrent(display, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT);
            if (eglSurface != EGL14.EGL_NO_SURFACE) EGL14.eglDestroySurface(display, eglSurface);
            EGL14.eglDestroyContext(display, context);
        }
        context = EGL14.EGL_NO_CONTEXT; eglSurface = EGL14.EGL_NO_SURFACE;
        program = 0; texture = 0;
        synchronized (lock) { ready = false; if (!reading) pixels = null; }
    }
}
