package dev.horizon.android;

/**
 * Screen recording for the in-game overlay: the consent prompt, the capture service's
 * state, and the recordings the player can play back.
 *
 * <p>Everything here is static and lives in the app's own process; the Rust client calls it
 * through JNI ({@code crates/horizon-client/src/android/recorder.rs}) and polls
 * {@link #status()} for progress. The heavy lifting is in {@link RecorderService}.
 */
public final class ScreenRecorder {
    /** The request code of the system's screen-capture consent prompt. */
    public static final int REQUEST_CONSENT = 4210;
    /** The request code of the microphone permission prompt. */
    public static final int REQUEST_MIC = 4211;

    /**
     * Where a finished recording lands in Android's shared storage: {@code Movies/VoltPlus}
     * (Android 10+ MediaStore rejects {@code Recordings} for {@code MediaStore.Video.Media}).
     */
    static final String FOLDER = android.os.Environment.DIRECTORY_MOVIES + "/VoltPlus";

    static java.io.File localRecordingsFolder(android.content.Context context) {
        java.io.File folder = new java.io.File(context.getFilesDir(), "recordings");
        if (!folder.exists()) {
            folder.mkdirs();
        }
        return folder;
    }

    /**
     * Copies a finished recording into the app's {@code recordings/} folder so it is visible in
     * both the Launcher File Manager and the Android Files provider in addition to the device's
     * Movies gallery.
     */
    static void mirrorToLocalRecordings(
            android.content.Context context,
            android.net.Uri mediaUri,
            java.io.File legacyFile,
            String displayName) {
        if (context == null || displayName == null || displayName.isEmpty()) {
            return;
        }
        java.io.File dest = new java.io.File(localRecordingsFolder(context), displayName);
        if (legacyFile != null && legacyFile.isFile()) {
            if (dest.getAbsolutePath().equals(legacyFile.getAbsolutePath())) {
                return;
            }
            try (java.io.FileInputStream in = new java.io.FileInputStream(legacyFile);
                    java.io.FileOutputStream out = new java.io.FileOutputStream(dest)) {
                byte[] buf = new byte[1 << 16];
                int n;
                while ((n = in.read(buf)) > 0) {
                    out.write(buf, 0, n);
                }
                out.flush();
            } catch (Throwable ignored) {
            }
            return;
        }
        if (mediaUri == null) {
            return;
        }
        try (java.io.InputStream in = context.getContentResolver().openInputStream(mediaUri);
                java.io.FileOutputStream out = new java.io.FileOutputStream(dest)) {
            if (in == null) {
                return;
            }
            byte[] buf = new byte[1 << 16];
            int n;
            while ((n = in.read(buf)) > 0) {
                out.write(buf, 0, n);
            }
            out.flush();
        } catch (Throwable ignored) {
        }
    }

    static volatile String state = "idle";
    static volatile String message = "";
    static volatile String uri = "";
    static volatile long startedAt = 0L;
    static volatile long bytes = 0L;
    /** The size and rate the encoder actually runs at, 0 until recording starts. */
    static volatile int runWidth = 0;
    static volatile int runHeight = 0;
    static volatile int runFps = 0;
    static volatile int width = 1280;
    static volatile int height = 720;
    static volatile int fps = 30;
    /** 0 no audio, 1 microphone, 2 the game's own audio. */
    static volatile int audioMode = 0;

    static volatile int consentResult = 0;
    static volatile android.content.Intent consentData = null;
    static volatile boolean rawGameplayOnly = false;

    private ScreenRecorder() {}

    /**
     * {@code sdk;displayWidth;displayHeight;inGameAudio;microphone;projection;device}: the facts
     * the overlay needs before it offers a combination.
     */
    public static String capabilities(android.content.Context context) {
        int sdk = android.os.Build.VERSION.SDK_INT;
        int width = 0;
        int height = 0;
        boolean microphone = false;
        boolean projection = false;
        String device = "unknown";
        // Every field is read on its own: a context that refuses one lookup must not
        // turn the whole answer into "this phone cannot record".
        try {
            if (context != null) {
                android.util.DisplayMetrics metrics = context.getResources().getDisplayMetrics();
                width = metrics.widthPixels;
                height = metrics.heightPixels;
            }
        } catch (Throwable ignored) {
        }
        try {
            if (context != null) {
                device = android.os.Build.MANUFACTURER + " " + android.os.Build.MODEL;
            }
        } catch (Throwable ignored) {
        }
        try {
            if (context != null) {
                microphone =
                        context.checkSelfPermission(android.Manifest.permission.RECORD_AUDIO)
                                == android.content.pm.PackageManager.PERMISSION_GRANTED;
            }
        } catch (Throwable ignored) {
        }
        try {
            if (context != null) {
                projection =
                        context.getSystemService(android.content.Context.MEDIA_PROJECTION_SERVICE)
                                != null;
            }
        } catch (Throwable ignored) {
        }
        return sdk
                + ";"
                + width
                + ";"
                + height
                + ";"
                + (sdk >= 29 ? 1 : 0)
                + ";"
                + (microphone ? 1 : 0)
                + ";"
                + (projection ? 1 : 0)
                + ";"
                + device;
    }

    /** Whether any recording is running: the display's, or the game's own frames. */
    static boolean busy() {
        return RecorderService.busy() || EngineRecorder.active();
    }

    /**
     * Records the game's own frames at {@code w x h}, {@code f} frames a second: no display
     * capture for no-audio/mic (no consent prompt), and the touch controls stay on screen without
     * appearing in the video (the renderer copies each frame into the encoder's surface before the
     * HUD). When {@code audio == 2} (in-game audio), requests MediaProjection only for audio
     * capture while keeping the video raw gameplay. Answers {@code state;message}.
     */
    public static synchronized String startEngine(
            android.app.Activity activity, int w, int h, int f, int audio) {
        if (activity == null) {
            return "error;no window to record";
        }
        if (busy()) {
            return "recording;already recording";
        }
        if (audio == 1
                && activity.getApplicationContext()
                                .checkSelfPermission(android.Manifest.permission.RECORD_AUDIO)
                        != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            try {
                activity.requestPermissions(
                        new String[] {android.Manifest.permission.RECORD_AUDIO}, REQUEST_MIC);
            } catch (Throwable ignored) {
            }
            return "permission;microphone access is off, allow it and start again";
        }
        if (audio == 2) {
            rawGameplayOnly = true;
            return startProjection(activity, w, h, f, 2);
        }
        rawGameplayOnly = false;
        uri = "";
        bytes = 0L;
        startedAt = System.currentTimeMillis();
        message = "";
        String answer =
                EngineRecorder.start(activity.getApplicationContext(), w, h, f, audio == 1);
        if (answer.startsWith("recording;")) {
            state = "recording";
        } else if (answer.startsWith("error;")) {
            state = "error";
            message = answer.substring("error;".length());
        } else if (answer.startsWith("unsupported;")) {
            state = "unsupported";
            message = answer.substring("unsupported;".length());
        }
        return answer;
    }

    /** The encoder's own surface, while the game's frames are being recorded. */
    public static synchronized android.view.Surface engineSurface() {
        return EngineRecorder.surface();
    }

    /** Records the CLOCK_MONOTONIC timestamp when the first frame hits the engine surface. */
    public static void noteEngineFrame() {
        EngineRecorder.noteFrameQueued();
    }

    /**
     * Asks for screen capture at {@code w x h}, {@code f} frames a second. The size is a request:
     * {@link RecorderService} fits it to the display and the device's encoder. Returns a
     * {@code state;message} pair the overlay shows as is.
     */
    public static synchronized String start(
            android.app.Activity activity, int w, int h, int f, int audio) {
        rawGameplayOnly = false;
        return startProjection(activity, w, h, f, audio);
    }

    private static synchronized String startProjection(
            android.app.Activity activity, int w, int h, int f, int audio) {
        if (activity == null) {
            return "error;no window to record";
        }
        if (busy()) {
            return "recording;already recording";
        }
        if ("asking".equals(state)) {
            // A second tap must not stack prompts: the first one's answer would arrive
            // as a cancel and look like a refusal.
            return "asking;the screen capture prompt is already open";
        }
        android.content.Context context = activity.getApplicationContext();
        if (audio < 0 || audio > 2) {
            audio = 0;
        }
        if (audio != 0
                && context.checkSelfPermission(android.Manifest.permission.RECORD_AUDIO)
                        != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            try {
                activity.requestPermissions(
                        new String[] {android.Manifest.permission.RECORD_AUDIO}, REQUEST_MIC);
            } catch (Throwable ignored) {
            }
            return "permission;microphone access is off, allow it and start again";
        }
        if (audio == 2 && android.os.Build.VERSION.SDK_INT < 29) {
            return "unsupported;in-game audio needs Android 10 or newer, choose microphone or none";
        }
        if (context.getSystemService(android.content.Context.MEDIA_PROJECTION_SERVICE) == null) {
            return "unsupported;this device has no screen capture service";
        }
        width = w;
        height = h;
        fps = f;
        audioMode = audio;
        uri = "";
        bytes = 0L;
        startedAt = 0L;
        message = "";
        try {
            android.media.projection.MediaProjectionManager manager =
                    (android.media.projection.MediaProjectionManager)
                            context.getSystemService(
                                    android.content.Context.MEDIA_PROJECTION_SERVICE);
            final android.content.Intent intent = manager.createScreenCaptureIntent();
            state = "asking";
            // The game's event loop is not the UI thread; the prompt has to open on it.
            activity.runOnUiThread(
                    new Runnable() {
                        public void run() {
                            try {
                                activity.startActivityForResult(intent, REQUEST_CONSENT);
                            } catch (Throwable error) {
                                state = "error";
                                message = shorter(error);
                            }
                        }
                    });
            return "asking;confirm the screen capture prompt";
        } catch (Throwable error) {
            state = "error";
            message = shorter(error);
            return "error;" + message;
        }
    }

    /** The system's answer to the consent prompt; the activity forwards it here. */
    public static synchronized void onActivityResult(
            android.app.Activity activity, int request, int result, android.content.Intent data) {
        if (request != REQUEST_CONSENT) {
            return;
        }
        if (result != android.app.Activity.RESULT_OK || data == null) {
            state = "denied";
            message =
                    "screen capture was not allowed (result "
                            + result
                            + (data == null ? ", no session" : "")
                            + " from "
                            + (activity == null ? "no activity" : activity.getClass().getSimpleName())
                            + ")";
            android.util.Log.w(
                    "VoltRecorder",
                    "consent result " + result + " data=" + (data != null) + " request=" + request);
            return;
        }
        consentResult = result;
        consentData = data;
        android.util.Log.w("VoltRecorder", "consent granted, starting the capture service");
        state = "starting";
        message = "";
        try {
            android.content.Intent service =
                    new android.content.Intent(activity, RecorderService.class);
            if (android.os.Build.VERSION.SDK_INT >= 26) {
                activity.startForegroundService(service);
            } else {
                activity.startService(service);
            }
        } catch (Throwable error) {
            state = "error";
            message = "could not start the recorder: " + shorter(error);
        }
    }

    /** Ends the current recording; the service finalises the file first. */
    public static synchronized String stop() {
        if (!busy()) {
            state = "idle";
            rawGameplayOnly = false;
            return "idle;nothing is recording";
        }
        if (RecorderService.current != null) {
            state = "stopping";
            RecorderService.stopNow("saved");
            return "stopping;finishing the file";
        }
        if (EngineRecorder.active()) {
            state = "stopping";
            return EngineRecorder.stop();
        }
        state = "stopping";
        RecorderService.stopNow("saved");
        return "stopping;finishing the file";
    }

    /** {@code state;millis;bytes;uri;width;height;fps;message}. */
    public static String status() {
        long millis = startedAt == 0L ? 0L : System.currentTimeMillis() - startedAt;
        return state
                + ";"
                + millis
                + ";"
                + bytes
                + ";"
                + uri
                + ";"
                + runWidth
                + ";"
                + runHeight
                + ";"
                + runFps
                + ";"
                + message;
    }

    /**
     * One recording per line, {@code name\tbytes\taddedMillis\turi}. On Android 10 and newer the
     * list is MediaStore's {@code Recordings} folder (plus the older {@code Movies/VoltPlus});
     * before that it is the folder the service writes to, where playing back needs Android 10.
     */
    public static String recordings(android.content.Context context) {
        StringBuilder out = new StringBuilder();
        if (context == null) {
            return "";
        }
        java.util.HashSet<String> seenNames = new java.util.HashSet<>();
        int seen = 0;
        if (android.os.Build.VERSION.SDK_INT >= 29) {
            android.net.Uri collection =
                    android.provider.MediaStore.Video.Media.getContentUri(
                            android.provider.MediaStore.VOLUME_EXTERNAL);
            String[] columns = {
                android.provider.MediaStore.Video.Media._ID,
                android.provider.MediaStore.Video.Media.DISPLAY_NAME,
                android.provider.MediaStore.Video.Media.SIZE,
                android.provider.MediaStore.Video.Media.DATE_ADDED,
            };
            String where =
                    "("
                            + android.provider.MediaStore.Video.Media.RELATIVE_PATH
                            + " LIKE ? OR "
                            + android.provider.MediaStore.Video.Media.RELATIVE_PATH
                            + " LIKE ?)";
            String[] args = {FOLDER + "%", "Recordings%"};
            android.database.Cursor cursor = null;
            try {
                cursor =
                        context.getContentResolver()
                                .query(
                                        collection,
                                        columns,
                                        where,
                                        args,
                                        android.provider.MediaStore.Video.Media.DATE_ADDED
                                                + " DESC");
                while (cursor != null && cursor.moveToNext() && seen < 40) {
                    long id = cursor.getLong(0);
                    String name = cursor.getString(1);
                    long size = cursor.getLong(2);
                    long added = cursor.getLong(3) * 1000L;
                    if (name == null || name.isEmpty() || size <= 0 || !seenNames.add(name)) {
                        continue;
                    }
                    android.net.Uri item =
                            android.content.ContentUris.withAppendedId(collection, id);
                    out.append(name)
                            .append('\t')
                            .append(size)
                            .append('\t')
                            .append(added)
                            .append('\t')
                            .append(item.toString())
                            .append('\n');
                    seen++;
                }
            } catch (Throwable ignored) {
            } finally {
                if (cursor != null) {
                    try {
                        cursor.close();
                    } catch (Throwable ignored) {
                    }
                }
            }
        }
        java.io.File[] folders = {
            localRecordingsFolder(context),
            RecorderService.legacyFolder(context),
        };
        java.util.ArrayList<java.io.File> localFiles = new java.util.ArrayList<>();
        for (java.io.File folder : folders) {
            java.io.File[] list = folder == null ? null : folder.listFiles();
            if (list != null) {
                for (java.io.File f : list) {
                    if (f.isFile() && f.length() > 0 && f.getName().endsWith(".mp4")) {
                        localFiles.add(f);
                    }
                }
            }
        }
        java.util.Collections.sort(
                localFiles,
                new java.util.Comparator<java.io.File>() {
                    public int compare(java.io.File a, java.io.File b) {
                        return Long.compare(b.lastModified(), a.lastModified());
                    }
                });
        for (java.io.File f : localFiles) {
            if (seen >= 40) {
                break;
            }
            if (!seenNames.add(f.getName())) {
                continue;
            }
            out.append(f.getName())
                    .append('\t')
                    .append(f.length())
                    .append('\t')
                    .append(f.lastModified())
                    .append("\tfile:")
                    .append(f.getAbsolutePath())
                    .append('\n');
            seen++;
        }
        return out.toString();
    }

    /** Opens a recording in the system's video player. */
    public static String play(android.app.Activity activity, String item) {
        if (activity == null || item == null || item.isEmpty()) {
            return "error;nothing to play";
        }
        try {
            android.net.Uri dataUri = android.net.Uri.parse(item);
            if (item.startsWith("file:")) {
                java.io.File file = new java.io.File(dataUri.getPath() == null ? "" : dataUri.getPath());
                java.io.File localDir = localRecordingsFolder(activity);
                if (file.getParentFile() != null
                        && file.getParentFile().getAbsolutePath().equals(localDir.getAbsolutePath())) {
                    dataUri =
                            android.provider.DocumentsContract.buildDocumentUri(
                                    activity.getPackageName() + ".documents",
                                    "recordings/" + file.getName());
                }
            }
            final android.content.Intent view =
                    new android.content.Intent(android.content.Intent.ACTION_VIEW);
            view.setDataAndType(dataUri, "video/mp4");
            view.addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.runOnUiThread(
                    new Runnable() {
                        public void run() {
                            try {
                                activity.startActivity(view);
                            } catch (Throwable error) {
                                message = shorter(error);
                            }
                        }
                    });
            return "ok;";
        } catch (Throwable error) {
            return "error;no video player could open it (" + shorter(error) + ")";
        }
    }

    static void publish(
            String nextState, String nextUri, long sizeBytes, long startMillis, String note) {
        state = nextState;
        uri = nextUri == null ? "" : nextUri;
        bytes = sizeBytes;
        startedAt = startMillis;
        message = note == null ? "" : note;
    }

    static String shorter(Throwable error) {
        String text = error.getClass().getSimpleName();
        String detail = error.getMessage();
        if (detail != null && !detail.isEmpty()) {
            text += ": " + detail;
        }
        return text.length() > 160 ? text.substring(0, 160) : text;
    }
}
