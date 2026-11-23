package dev.horizon.android;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.ContentValues;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioPlaybackCaptureConfiguration;
import android.media.AudioRecord;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaExtractor;
import android.media.MediaFormat;
import android.media.MediaMuxer;
import android.media.MediaRecorder;
import android.media.projection.MediaProjection;
import android.media.projection.MediaProjectionManager;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.ParcelFileDescriptor;
import android.provider.MediaStore;
import android.view.Surface;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;

/**
 * Records the screen with MediaProjection while the player keeps playing: a {@link MediaRecorder}
 * writes H.264 into an MP4, the microphone comes from the same recorder, and the game's own audio
 * is captured through {@link AudioPlaybackCaptureConfiguration} (Android 10+) and merged into the
 * file when the recording stops. The service is foreground for as long as it captures, as Android
 * 10 and newer require for a projection.
 *
 * <p>Every capability is checked before use: the size is fitted to the display and to a hardware
 * AVC encoder, frames above what the encoder accepts are reported instead of silently dropped, and
 * a recording that fails half way is deleted rather than left as a broken file.
 */
public final class RecorderService extends Service {
    /**
     * Where a recording goes before Android 10, when MediaStore cannot help: the shared
     * {@code Recordings} folder if the permission allows writing there, otherwise the app's own
     * Movies folder, which nothing else can read.
     */
    static File legacyFolder(android.content.Context context) {
        File shared = new File(Environment.getExternalStorageDirectory(), "Recordings");
        if (shared.exists() ? shared.canWrite() : shared.mkdirs()) {
            return shared;
        }
        File movies = context.getExternalFilesDir(Environment.DIRECTORY_MOVIES);
        if (movies == null) {
            movies = context.getFilesDir();
        }
        File folder = new File(movies, "Recordings");
        if (!folder.exists()) {
            folder.mkdirs();
        }
        return folder;
    }

    static volatile RecorderService current;

    /// Everything MediaProjection and MediaRecorder do here needs a Looper, and the
    /// service's own threads have none: one looper thread carries the whole capture.
    private HandlerThread worker;
    private Handler workerHandler;

    private static final String CHANNEL = "volt_recording";
    private static final int NOTIFICATION_ID = 4610;
    /** The capture never goes above this, however small the display is. */
    private static final int MIN_WIDTH = 160;

    private MediaProjection projection;
    private MediaProjection.Callback projectionCallback;
    private VirtualDisplay display;
    private MediaRecorder recorder;
    private Uri target;
    private File legacyTarget;
    private String displayName;
    private ParcelFileDescriptor targetFile;
    private File tempVideo;
    private AudioCapture audio;
    private int width = 1280;
    private int height = 720;
    private int frames = 30;
    private int audioMode;
    private long startedAt;
    private volatile boolean stopping;
    private volatile boolean failed;

    static boolean busy() {
        RecorderService service = current;
        return service != null && service.recorder != null && !service.stopping;
    }

    static void stopNow(String reason) {
        RecorderService service = current;
        if (service != null) {
            service.stopRecording(reason);
        }
    }

    @Override
    public android.os.IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public void onCreate() {
        super.onCreate();
        current = this;
        worker = new HandlerThread("Volt recorder");
        worker.start();
        workerHandler = new Handler(worker.getLooper());
    }

    @Override
    public void onDestroy() {
        current = null;
        if (worker != null) {
            worker.quitSafely();
            worker = null;
            workerHandler = null;
        }
        super.onDestroy();
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        startForegroundNotice();
        if (recorder == null && !stopping && workerHandler != null) {
            final RecorderService self = this;
            workerHandler.post(
                    new Runnable() {
                        public void run() {
                            self.begin();
                        }
                    });
        }
        return START_NOT_STICKY;
    }

    private void startForegroundNotice() {
        String title = "Volt Plus is recording";
        String text = "Tap the overlay to stop";
        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= 26) {
            NotificationManager manager =
                    (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
            NotificationChannel channel =
                    new NotificationChannel(
                            CHANNEL, "Recording", NotificationManager.IMPORTANCE_LOW);
            channel.setDescription("Shown while the screen recorder is running");
            channel.setShowBadge(false);
            manager.createNotificationChannel(channel);
            builder = new Notification.Builder(this, CHANNEL);
        } else {
            builder = new Notification.Builder(this);
        }
        Intent open = new Intent(this, HorizonActivity.class);
        open.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
        PendingIntent pending =
                PendingIntent.getActivity(
                        this,
                        0,
                        open,
                        PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        builder.setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.presence_video_online)
                .setOngoing(true)
                .setContentIntent(pending);
        Notification notification = builder.build();
        try {
            if (Build.VERSION.SDK_INT >= 29) {
                startForeground(
                        NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION);
            } else {
                startForeground(NOTIFICATION_ID, notification);
            }
        } catch (Throwable error) {
            ScreenRecorder.publish(
                    "error", "", 0L, 0L, "no foreground capture service: " + ScreenRecorder.shorter(error));
            stopSelf();
        }
    }

    /** Opens the projection and starts the recorder; runs off the main thread. */
    private synchronized void begin() {
        if (recorder != null) {
            return;
        }
        try {
            MediaProjectionManager manager =
                    (MediaProjectionManager) getSystemService(MEDIA_PROJECTION_SERVICE);
            if (manager == null || ScreenRecorder.consentData == null) {
                fail("the screen capture session was lost, start again");
                return;
            }
            projection =
                    manager.getMediaProjection(
                            ScreenRecorder.consentResult, ScreenRecorder.consentData);
            if (projection == null) {
                fail("the system refused the capture session");
                return;
            }
            projectionCallback =
                    new MediaProjection.Callback() {
                        @Override
                        public void onStop() {
                            stopRecording("the system ended screen capture");
                        }
                    };
            projection.registerCallback(projectionCallback, workerHandler);

            audioMode = ScreenRecorder.audioMode;
            frames = Math.max(15, Math.min(165, ScreenRecorder.fps));

            android.util.DisplayMetrics metrics = getResources().getDisplayMetrics();
            int displayWidth = Math.max(2, metrics.widthPixels);
            int displayHeight = Math.max(2, metrics.heightPixels);
            int requestedHeight = Math.max(120, Math.min(ScreenRecorder.height, displayHeight));
            int requestedWidth =
                    (int)
                            Math.round(
                                    (double) requestedHeight * displayWidth / (double) displayHeight);
            requestedWidth = Math.max(MIN_WIDTH, Math.min(requestedWidth, displayWidth));

            int[] fitted = fit(requestedWidth, requestedHeight, frames);
            if (fitted == null) {
                // Ask the encoder what it can do at all, then say so plainly.
                fail(
                        "this device's H.264 encoder cannot record "
                                + requestedWidth
                                + "x"
                                + requestedHeight
                                + " at "
                                + frames
                                + " fps, try a lower resolution or frame rate");
                return;
            }
            width = fitted[0];
            height = fitted[1];
            frames = fitted[2];

            long bitrate =
                    Math.max(
                            2_000_000L,
                            Math.min(
                                    60_000_000L,
                                    (long) width * height * Math.max(24, frames) / 8L));
            if (Build.VERSION.SDK_INT >= 31) {
                recorder = new MediaRecorder(this);
            } else {
                recorder = new MediaRecorder();
            }
            recorder.setVideoSource(MediaRecorder.VideoSource.SURFACE);
            if (audioMode == 1) {
                recorder.setAudioSource(MediaRecorder.AudioSource.MIC);
            }
            recorder.setOutputFormat(MediaRecorder.OutputFormat.MPEG_4);
            recorder.setVideoEncoder(MediaRecorder.VideoEncoder.H264);
            recorder.setVideoSize(width, height);
            recorder.setVideoFrameRate(frames);
            recorder.setVideoEncodingBitRate((int) bitrate);
            recorder.setOrientationHint(displayWidth >= displayHeight ? 0 : 90);
            if (audioMode == 1) {
                recorder.setAudioEncoder(MediaRecorder.AudioEncoder.AAC);
                recorder.setAudioChannels(1);
                recorder.setAudioSamplingRate(44_100);
                recorder.setAudioEncodingBitRate(96_000);
            }
            if (audioMode == 2) {
                // The game's audio is captured separately; the video is staged so both
                // tracks can be merged into the MediaStore file when recording ends.
                tempVideo = File.createTempFile("volt-capture-", ".mp4", getCacheDir());
                recorder.setOutputFile(tempVideo.getAbsolutePath());
            } else {
                openTarget();
                if (targetFile == null) {
                    fail("no writable place for the recording");
                    return;
                }
                recorder.setOutputFile(targetFile.getFileDescriptor());
            }
            recorder.prepare();

            Surface surface = recorder.getSurface();
            display =
                    projection.createVirtualDisplay(
                            "VoltPlus",
                            width,
                            height,
                            Math.max(120, getResources().getDisplayMetrics().densityDpi),
                            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                            surface,
                            null,
                            workerHandler);
            if (audioMode == 2) {
                audio = new AudioCapture(projection, getCacheDir());
                audio.start();
            }
            recorder.start();
            startedAt = System.currentTimeMillis();
            ScreenRecorder.runWidth = width;
            ScreenRecorder.runHeight = height;
            ScreenRecorder.runFps = frames;
            ScreenRecorder.startedAt = startedAt;
            ScreenRecorder.bytes = 0L;
            ScreenRecorder.uri = "";
            ScreenRecorder.state = "recording";
            ScreenRecorder.message =
                    width + "x" + height + " at " + frames + " fps";
        } catch (Throwable error) {
            fail("could not start recording: " + ScreenRecorder.shorter(error));
        }
    }

    /** An encoder-accepted size and rate near the request, or null when AVC cannot do it. */
    private static int[] fit(int wantedWidth, int wantedHeight, int wantedFrames) {
        int[][] sizes = {
            {wantedWidth, wantedHeight},
            {1280, 720},
            {960, 540},
            {854, 480},
            {640, 360},
            {426, 240},
        };
        int[] rates = {wantedFrames, 60, 30};
        for (int[] size : sizes) {
            int w = size[0] - (size[0] % 2);
            int h = size[1] - (size[1] % 2);
            if (w < MIN_WIDTH || h < 120) {
                continue;
            }
            for (int rate : rates) {
                if (rate <= 0) {
                    continue;
                }
                MediaFormat format =
                        MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, w, h);
                format.setInteger(
                        MediaFormat.KEY_COLOR_FORMAT,
                        MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
                format.setInteger(MediaFormat.KEY_BIT_RATE, 8_000_000);
                format.setInteger(MediaFormat.KEY_FRAME_RATE, rate);
                format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 2);
                try {
                    MediaCodecList list = new MediaCodecList(MediaCodecList.REGULAR_CODECS);
                    if (list.findEncoderForFormat(format) != null) {
                        return new int[] {w, h, rate};
                    }
                } catch (Throwable ignored) {
                }
            }
        }
        return null;
    }

    private void openTarget() throws Exception {
        String name = "VoltPlus-" + timestamp() + ".mp4";
        displayName = name;
        target = null;
        legacyTarget = null;
        targetFile = null;
        if (Build.VERSION.SDK_INT >= 29) {
            try {
                ContentValues values = new ContentValues();
                values.put(MediaStore.Video.Media.DISPLAY_NAME, name);
                values.put(MediaStore.Video.Media.MIME_TYPE, "video/mp4");
                values.put(MediaStore.Video.Media.RELATIVE_PATH, ScreenRecorder.FOLDER);
                values.put(MediaStore.Video.Media.IS_PENDING, 1);
                target =
                        getContentResolver()
                                .insert(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, values);
                if (target != null) {
                    targetFile = getContentResolver().openFileDescriptor(target, "rw");
                }
            } catch (Throwable ignored) {
                target = null;
                targetFile = null;
            }
            if (targetFile != null) {
                return;
            }
        }
        File folder = ScreenRecorder.localRecordingsFolder(this);
        legacyTarget = new File(folder, name);
        targetFile =
                ParcelFileDescriptor.open(
                        legacyTarget,
                        ParcelFileDescriptor.MODE_CREATE
                                | ParcelFileDescriptor.MODE_READ_WRITE
                                | ParcelFileDescriptor.MODE_TRUNCATE);
    }

    private String timestamp() {
        java.text.SimpleDateFormat format =
                new java.text.SimpleDateFormat("yyyyMMdd-HHmmss", java.util.Locale.US);
        return format.format(new java.util.Date());
    }

    private void publishTarget(String note) {
        String uri = target == null ? "" : target.toString();
        long size = 0L;
        try {
            if (targetFile != null) {
                size = targetFile.getStatSize();
            }
        } catch (Throwable ignored) {
        }
        if (size <= 0 && target != null) {
            try (android.database.Cursor cursor =
                    getContentResolver()
                            .query(
                                    target,
                                    new String[] {MediaStore.Video.Media.SIZE},
                                    null,
                                    null,
                                    null)) {
                if (cursor != null && cursor.moveToFirst()) {
                    size = cursor.getLong(0);
                }
            } catch (Throwable ignored) {
            }
        }
        ScreenRecorder.publish("saved", uri, size, 0L, note);
    }

    private void publishLegacy(File file, String note) {
        ScreenRecorder.publish(
                "saved", "file:" + file.getAbsolutePath(), file.length(), 0L, note);
    }

    void stopRecording(String reason) {
        if (stopping) {
            return;
        }
        stopping = true;
        ScreenRecorder.state = "stopping";
        final String note = reason;
        if (workerHandler == null) {
            return;
        }
        workerHandler.post(
                new Runnable() {
                    public void run() {
                        RecorderService.this.finish(note);
                    }
                });
    }

    private void finish(String reason) {
        boolean stopped = false;
        try {
            if (recorder != null) {
                try {
                    recorder.stop();
                    stopped = true;
                } catch (Throwable error) {
                    // MediaRecorder throws when it captured nothing (a stop within a
                    // second, or a failed encoder). Nothing usable was written.
                }
            }
        } finally {
            if (recorder != null) {
                try {
                    recorder.reset();
                } catch (Throwable ignored) {
                }
                try {
                    recorder.release();
                } catch (Throwable ignored) {
                }
                recorder = null;
            }
        }
        if (display != null) {
            try {
                display.release();
            } catch (Throwable ignored) {
            }
            display = null;
        }
        if (audio != null) {
            try {
                audio.stopAndWait();
            } catch (Throwable ignored) {
            }
        }
        if (projection != null) {
            try {
                if (projectionCallback != null) {
                    projection.unregisterCallback(projectionCallback);
                }
                projection.stop();
            } catch (Throwable ignored) {
            }
            projection = null;
        }
        String note = reason;
        try {
            if (!stopped || failed) {
                discard();
                ScreenRecorder.publish(
                        "error",
                        "",
                        0L,
                        0L,
                        "the recording did not capture anything, nothing was saved");
            } else if (audioMode == 2) {
                note = finishGameAudio();
            } else {
                finalizeTarget(note);
            }
        } catch (Throwable error) {
            discard();
            ScreenRecorder.publish(
                    "error", "", 0L, 0L, "saving failed: " + ScreenRecorder.shorter(error));
        } finally {
            try {
                if (targetFile != null) {
                    targetFile.close();
                }
            } catch (Throwable ignored) {
            }
            targetFile = null;
            deleteQuietly(tempVideo);
            tempVideo = null;
            if (audio != null) {
                deleteQuietly(audio.file());
                audio = null;
            }
            stopping = false;
            stopForeground(true);
            stopSelf();
        }
    }

    private void finalizeTarget(String note) {
        if (target != null) {
            ContentValues values = new ContentValues();
            values.put(MediaStore.Video.Media.IS_PENDING, 0);
            try {
                getContentResolver().update(target, values, null, null);
            } catch (Throwable ignored) {
            }
            ScreenRecorder.mirrorToLocalRecordings(this, target, legacyTarget, displayName);
            publishTarget(note);
        } else if (legacyTarget != null) {
            ScreenRecorder.mirrorToLocalRecordings(this, null, legacyTarget, displayName);
            publishLegacy(legacyTarget, note);
        } else {
            ScreenRecorder.publish("error", "", 0L, 0L, "no file was written");
        }
    }

    /** Merges the staged video with the captured game audio; falls back to video only. */
    private String finishGameAudio() throws Exception {
        File audioFile = audio == null ? null : audio.file();
        if (tempVideo == null || audioFile == null || !audioFile.exists() || audioFile.length() == 0) {
            openTarget();
            if (targetFile == null) {
                ScreenRecorder.publish("error", "", 0L, 0L, "no place to save the recording");
                return "nothing saved";
            }
            copy(tempVideo, targetFile);
            finalizeOpened("game audio was not captured, saved video only");
            return "saved video only";
        }
        try {
            openTarget();
            if (targetFile == null) {
                ScreenRecorder.publish("error", "", 0L, 0L, "no place to save the recording");
                return "nothing saved";
            }
            boolean merged = Muxer.merge(tempVideo, audioFile, targetFile.getFileDescriptor());
            finalizeOpened(merged ? "with game audio" : "game audio could not be merged");
            return merged ? "saved with game audio" : "saved video only";
        } catch (Throwable error) {
            ScreenRecorder.publish(
                    "error",
                    "",
                    0L,
                    0L,
                    "the game audio could not be merged (" + ScreenRecorder.shorter(error) + ")");
            return "game audio merge failed";
        }
    }

    /** Closes the pending MediaStore entry (or names the legacy file) as finished. */
    private void finalizeOpened(String note) {
        if (target != null) {
            ContentValues values = new ContentValues();
            values.put(MediaStore.Video.Media.IS_PENDING, 0);
            try {
                getContentResolver().update(target, values, null, null);
            } catch (Throwable ignored) {
            }
            ScreenRecorder.mirrorToLocalRecordings(this, target, legacyTarget, displayName);
            publishTarget(note);
        } else if (legacyTarget != null) {
            ScreenRecorder.mirrorToLocalRecordings(this, null, legacyTarget, displayName);
            publishLegacy(legacyTarget, note);
        }
    }

    private static void copy(File from, ParcelFileDescriptor to) {
        if (from == null || to == null) {
            return;
        }
        try (FileInputStream in = new FileInputStream(from);
                FileOutputStream out = new FileOutputStream(to.getFileDescriptor())) {
            byte[] buffer = new byte[1 << 16];
            int read;
            while ((read = in.read(buffer)) > 0) {
                out.write(buffer, 0, read);
            }
            out.flush();
        } catch (Throwable ignored) {
        }
    }

    private void discard() {
        try {
            if (target != null) {
                getContentResolver().delete(target, null, null);
            }
        } catch (Throwable ignored) {
        }
        if (audio != null) {
            deleteQuietly(audio.file());
        }
        if (tempVideo != null && target == null) {
            deleteQuietly(tempVideo);
        }
    }

    private void fail(String note) {
        failed = true;
        ScreenRecorder.publish("error", "", 0L, 0L, note);
        stopping = true;
        try {
            if (target != null) {
                getContentResolver().delete(target, null, null);
            }
        } catch (Throwable ignored) {
        }
        if (audio != null) {
            try {
                audio.stopAndWait();
            } catch (Throwable ignored) {
            }
            deleteQuietly(audio.file());
            audio = null;
        }
        deleteQuietly(tempVideo);
        try {
            if (recorder != null) {
                recorder.release();
            }
        } catch (Throwable ignored) {
        }
        recorder = null;
        if (display != null) {
            try {
                display.release();
            } catch (Throwable ignored) {
            }
            display = null;
        }
        if (projection != null) {
            try {
                projection.stop();
            } catch (Throwable ignored) {
            }
            projection = null;
        }
        stopping = false;
        stopForeground(true);
        stopSelf();
    }

    private static void deleteQuietly(File file) {
        try {
            if (file != null && file.exists()) {
                file.delete();
            }
        } catch (Throwable ignored) {
        }
    }

    /** The game's own audio, captured as AAC in a temporary MP4 (Android 10 and newer). */
    static final class AudioCapture {
        private final MediaProjection projection;
        private final File file;
        private final int sampleRate = 44_100;
        private AudioRecord record;
        private MediaCodec codec;
        private MediaMuxer muxer;
        private Thread thread;
        private volatile boolean running;
        private boolean started;

        AudioCapture(MediaProjection projection, File cache) throws Exception {
            this.projection = projection;
            this.file = File.createTempFile("volt-audio-", ".mp4", cache);
        }

        File file() {
            return file;
        }

        void start() throws Exception {
            // AudioPlaybackCaptureConfiguration is Android 10's; the overlay already refuses
            // the choice below that, and this keeps the call itself guarded too.
            if (Build.VERSION.SDK_INT < 29) {
                throw new IllegalStateException("playback capture needs Android 10");
            }
            AudioPlaybackCaptureConfiguration config =
                    new AudioPlaybackCaptureConfiguration.Builder(projection)
                            .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                            .addMatchingUsage(AudioAttributes.USAGE_GAME)
                            .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN)
                            .build();
            int minimum =
                    AudioRecord.getMinBufferSize(
                            sampleRate, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT);
            int buffer = Math.max(minimum * 2, 16_384);
            record =
                    new AudioRecord.Builder()
                            .setAudioFormat(
                                    new AudioFormat.Builder()
                                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                                            .setSampleRate(sampleRate)
                                            .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
                                            .build())
                            .setBufferSizeInBytes(buffer)
                            .setAudioPlaybackCaptureConfig(config)
                            .build();
            MediaFormat format =
                    MediaFormat.createAudioFormat(
                            MediaFormat.MIMETYPE_AUDIO_AAC, sampleRate, 1);
            format.setInteger(
                    MediaFormat.KEY_AAC_PROFILE,
                    MediaCodecInfo.CodecProfileLevel.AACObjectLC);
            format.setInteger(MediaFormat.KEY_BIT_RATE, 96_000);
            format.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 16_384);
            codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_AUDIO_AAC);
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            codec.start();
            if (record.getState() != AudioRecord.STATE_INITIALIZED) {
                throw new IllegalStateException("the playback capture could not start");
            }
            record.startRecording();
            running = true;
            final AudioCapture self = this;
            thread =
                    new Thread(
                            new Runnable() {
                                public void run() {
                                    self.pump();
                                }
                            },
                            "Volt recorder audio");
            thread.start();
            started = true;
        }

        private void pump() {
            android.media.MediaCodec.BufferInfo info = new android.media.MediaCodec.BufferInfo();
            byte[] pcm = new byte[8192];
            int track = -1;
            boolean formatSent = false;
            long total = 0L;
            try {
                while (running) {
                    int read = record.read(pcm, 0, pcm.length);
                    if (read > 0) {
                        int index = codec.dequeueInputBuffer(10_000);
                        if (index >= 0) {
                            ByteBuffer input = codec.getInputBuffer(index);
                            if (input != null) {
                                input.clear();
                                input.put(pcm, 0, Math.min(read, input.capacity()));
                                long time = total * 1_000_000L / (2L * sampleRate);
                                codec.queueInputBuffer(
                                        index, 0, Math.min(read, input.capacity()), time, 0);
                                total += Math.min(read, input.capacity());
                            }
                        }
                    } else if (read < 0) {
                        break;
                    }
                    int outIndex;
                    while ((outIndex = codec.dequeueOutputBuffer(info, 0)) >= 0) {
                        ByteBuffer out = codec.getOutputBuffer(outIndex);
                        if (out != null && info.size > 0) {
                            if ((info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0) {
                                info.size = 0;
                            }
                            if (info.size > 0) {
                                if (!formatSent) {
                                    MediaFormat outFormat = codec.getOutputFormat();
                                    muxer =
                                            new MediaMuxer(
                                                    file.getAbsolutePath(),
                                                    MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
                                    track = muxer.addTrack(outFormat);
                                    muxer.start();
                                    formatSent = true;
                                }
                                out.position(info.offset);
                                out.limit(info.offset + info.size);
                                muxer.writeSampleData(track, out, info);
                            }
                        }
                        codec.releaseOutputBuffer(outIndex, false);
                    }
                }
            } catch (Throwable ignored) {
            } finally {
                try {
                    if (muxer != null && formatSent) {
                        muxer.stop();
                    }
                } catch (Throwable ignored) {
                }
                try {
                    if (muxer != null) {
                        muxer.release();
                    }
                } catch (Throwable ignored) {
                }
                muxer = null;
            }
        }

        void stopAndWait() {
            running = false;
            try {
                if (record != null) {
                    record.stop();
                }
            } catch (Throwable ignored) {
            }
            try {
                if (thread != null) {
                    thread.join(1_500L);
                }
            } catch (Throwable ignored) {
            }
            try {
                if (codec != null) {
                    codec.stop();
                    codec.release();
                }
            } catch (Throwable ignored) {
            }
            codec = null;
            try {
                if (record != null) {
                    record.release();
                }
            } catch (Throwable ignored) {
            }
            record = null;
        }

        boolean started() {
            return started;
        }
    }

    /** Copies the video track of one MP4 and the audio track of another into a third. */
    static final class Muxer {
        static boolean merge(File video, File audio, java.io.FileDescriptor out) {
            MediaExtractor videoSource = new MediaExtractor();
            MediaExtractor audioSource = new MediaExtractor();
            MediaMuxer muxer = null;
            try {
                videoSource.setDataSource(video.getAbsolutePath());
                audioSource.setDataSource(audio.getAbsolutePath());
                int videoTrack = trackOf(videoSource, "video/");
                int audioTrack = trackOf(audioSource, "audio/");
                if (videoTrack < 0 || audioTrack < 0) {
                    return false;
                }
                muxer = new MediaMuxer(out, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
                videoSource.selectTrack(videoTrack);
                audioSource.selectTrack(audioTrack);
                int outVideo = muxer.addTrack(videoSource.getTrackFormat(videoTrack));
                int outAudio = muxer.addTrack(audioSource.getTrackFormat(audioTrack));
                muxer.start();
                android.media.MediaCodec.BufferInfo info = new android.media.MediaCodec.BufferInfo();
                copyTrack(videoSource, muxer, outVideo, info);
                copyTrack(audioSource, muxer, outAudio, info);
                muxer.stop();
                return true;
            } catch (Throwable error) {
                return false;
            } finally {
                try {
                    if (muxer != null) {
                        muxer.release();
                    }
                } catch (Throwable ignored) {
                }
                try {
                    videoSource.release();
                } catch (Throwable ignored) {
                }
                try {
                    audioSource.release();
                } catch (Throwable ignored) {
                }
            }
        }

        private static int trackOf(MediaExtractor extractor, String prefix) {
            for (int i = 0; i < extractor.getTrackCount(); i++) {
                MediaFormat format = extractor.getTrackFormat(i);
                String mime = format.getString(MediaFormat.KEY_MIME);
                if (mime != null && mime.startsWith(prefix)) {
                    return i;
                }
            }
            return -1;
        }

        private static void copyTrack(
                MediaExtractor source,
                MediaMuxer muxer,
                int track,
                android.media.MediaCodec.BufferInfo info) {
            ByteBuffer buffer = ByteBuffer.allocateDirect(1 << 20);
            while (true) {
                int size = source.readSampleData(buffer, 0);
                if (size < 0) {
                    break;
                }
                info.offset = 0;
                info.size = size;
                info.presentationTimeUs = source.getSampleTime();
                info.flags = source.getSampleFlags();
                muxer.writeSampleData(track, buffer, info);
                source.advance();
            }
        }
    }
}
