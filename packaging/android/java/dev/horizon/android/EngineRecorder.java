package dev.horizon.android;

import android.content.ContentValues;
import android.content.Context;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaFormat;
import android.media.MediaMuxer;
import android.net.Uri;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.provider.MediaStore;
import android.view.Surface;
import java.io.File;
import java.nio.ByteBuffer;

/**
 * Records the game's own frames: a {@link MediaCodec} H.264 encoder with an input {@link
 * Surface}, and a {@link MediaMuxer} writing the same {@code Recordings} MP4 a screen
 * recording makes.
 *
 * <p>This is what makes a raw recording possible: the renderer copies every frame into the
 * encoder's surface <em>before</em> the HUD is drawn, so the video has the game without the
 * touch controls while the screen keeps them. No MediaProjection (and so no consent prompt)
 * is involved: the game hands the frames over itself.
 *
 * <p>Video only. Recording audio needs the display capture service, so the overlay keeps
 * using {@link RecorderService} when an audio source is chosen.
 */
final class EngineRecorder {
    private static MediaCodec codec;
    private static MediaMuxer muxer;
    private static Surface surface;
    private static Thread drain;
    private static Uri target;
    private static File legacyTarget;
    private static String displayName;
    private static ParcelFileDescriptor targetFile;
    private static int track = -1;
    private static int width;
    private static int height;
    private static int frames;
    private static long startedAt;
    private static long written;
    private static long lastPublish;
    private static boolean started;
    private static volatile boolean stopping;
    /** The application context, for the MediaStore entry the recording closes. */
    private static Context appContext;

    private EngineRecorder() {}

    /** The surface the game renders into, or {@code null}. */
    static synchronized Surface surface() {
        return surface;
    }

    /** Taking frames right now. */
    static synchronized boolean busy() {
        return started && !stopping;
    }

    /** Holds the encoder, whether or not it still takes frames (it finalises the file). */
    static synchronized boolean active() {
        return started;
    }

    /**
     * Opens the encoder and its file. Answers {@code recording;WxH} with the size the device
     * accepted, or {@code error;…} / {@code unsupported;…} with the reason.
     */
    static synchronized String start(Context context, int wantedWidth, int wantedHeight, int wantedFps) {
        if (started) {
            return "recording;already recording";
        }
        int[] fit = fit(wantedWidth, wantedHeight, wantedFps);
        if (fit == null) {
            return "unsupported;this device has no H.264 encoder for that size";
        }
        width = fit[0];
        height = fit[1];
        frames = fit[2];
        appContext = context.getApplicationContext() == null ? context : context.getApplicationContext();
        MediaCodec encoder = null;
        Surface input = null;
        MediaMuxer out = null;
        try {
            MediaFormat format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height);
            format.setInteger(
                    MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
            format.setInteger(MediaFormat.KEY_BIT_RATE, bitrate(width, height, frames));
            format.setInteger(MediaFormat.KEY_FRAME_RATE, frames);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
            encoder = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC);
            encoder.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            input = encoder.createInputSurface();
            openTarget(context);
            if (targetFile == null) {
                throw new IllegalStateException("the recordings folder is not writable");
            }
            if (legacyTarget != null) {
                out = new MediaMuxer(
                        legacyTarget.getAbsolutePath(), MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
            } else {
                out = new MediaMuxer(
                        targetFile.getFileDescriptor(), MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
            }
            encoder.start();
        } catch (Throwable error) {
            closeQuietly(encoder, input, out);
            deleteTarget(context);
            ScreenRecorder.publish("error", "", 0L, 0L, ScreenRecorder.shorter(error));
            return "error;" + ScreenRecorder.shorter(error);
        }
        codec = encoder;
        surface = input;
        muxer = out;
        track = -1;
        stopping = false;
        started = true;
        written = 0L;
        startedAt = System.currentTimeMillis();
        lastPublish = 0L;
        ScreenRecorder.runWidth = width;
        ScreenRecorder.runHeight = height;
        ScreenRecorder.runFps = frames;
        Thread thread = new Thread(EngineRecorder::drain, "Volt engine recorder");
        thread.setDaemon(true);
        drain = thread;
        thread.start();
        return "recording;" + width + "x" + height;
    }

    /**
     * Ends the recording. The encoder is finalised on its own thread and the overlay hears
     * {@code saved} when the file is whole: the game must not wait for a codec.
     */
    static String stop() {
        synchronized (EngineRecorder.class) {
            if (!started) {
                return "idle;nothing is recording";
            }
            if (stopping) {
                return "stopping;finishing the file";
            }
            stopping = true;
        }
        Thread finisher = new Thread(EngineRecorder::finish, "Volt engine recorder stop");
        finisher.setDaemon(true);
        finisher.start();
        return "stopping;finishing the file";
    }

    /** Drains the last frames, closes the codec and finalises the MP4. Never on the game. */
    private static void finish() {
        MediaCodec encoder;
        Thread worker;
        synchronized (EngineRecorder.class) {
            encoder = codec;
            worker = drain;
            drain = null;
        }
        if (encoder != null) {
            try {
                encoder.signalEndOfInputStream();
            } catch (Throwable ignored) {
            }
        }
        // Let the drain thread take the last frames before the encoder goes away.
        if (worker != null && worker != Thread.currentThread()) {
            try {
                worker.join(3000);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
        }
        Surface input;
        synchronized (EngineRecorder.class) {
            codec = null;
            input = surface;
            surface = null;
        }
        if (encoder != null) {
            try {
                encoder.stop();
            } catch (Throwable ignored) {
            }
            try {
                encoder.release();
            } catch (Throwable ignored) {
            }
        }
        if (input != null) {
            try {
                input.release();
            } catch (Throwable ignored) {
            }
        }
        boolean finished = finishMuxer();
        long size = size();
        Uri savedMedia = target;
        File savedLegacy = legacyTarget;
        String savedName = displayName;
        String uri =
                savedMedia != null
                        ? savedMedia.toString()
                        : (savedLegacy != null ? "file:" + savedLegacy.getAbsolutePath() : "");
        boolean empty = written == 0L;
        closeTarget();
        if (!empty && appContext != null) {
            ScreenRecorder.mirrorToLocalRecordings(appContext, savedMedia, savedLegacy, savedName);
        }
        legacyTarget = null;
        displayName = null;
        synchronized (EngineRecorder.class) {
            started = false;
            stopping = false;
        }
        if (empty) {
            // Nothing reached the encoder: say so rather than hand over an empty file.
            ScreenRecorder.publish(
                    "error",
                    "",
                    0L,
                    0L,
                    "no frames reached the encoder: this device would not record the game's own surface");
            return;
        }
        ScreenRecorder.publish(
                "saved",
                uri,
                size,
                startedAt,
                finished ? "" : "the recording may be incomplete");
    }

    /** The drain loop: encoder output to the muxer, until the end of the stream. */
    private static void drain() {
        MediaCodec encoder;
        synchronized (EngineRecorder.class) {
            encoder = codec;
        }
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        if (encoder == null) {
            return;
        }
        try {
            while (true) {
                int index = encoder.dequeueOutputBuffer(info, 10_000);
                if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    startMuxer(encoder);
                } else if (index >= 0) {
                    ByteBuffer buffer = encoder.getOutputBuffer(index);
                    if (buffer != null
                            && info.size > 0
                            && (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0) {
                        write(buffer, info);
                    }
                    encoder.releaseOutputBuffer(index, false);
                }
                if ((info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0) {
                    break;
                }
                if (stopping && index == MediaCodec.INFO_TRY_AGAIN_LATER) {
                    // Nothing left to drain once the encoder was told to finish.
                    break;
                }
            }
        } catch (Throwable error) {
            ScreenRecorder.publish("error", "", 0L, 0L, ScreenRecorder.shorter(error));
        }
    }

    private static void startMuxer(MediaCodec encoder) {
        if (stopping) {
            return;
        }
        synchronized (EngineRecorder.class) {
            if (track >= 0 || muxer == null) {
                return;
            }
            track = muxer.addTrack(encoder.getOutputFormat());
            muxer.start();
        }
    }

    private static void write(ByteBuffer buffer, MediaCodec.BufferInfo info) {
        // `stop` holds the lock while it joins this thread: never wait for it here.
        if (stopping) {
            return;
        }
        synchronized (EngineRecorder.class) {
            if (track < 0 || muxer == null) {
                return;
            }
            buffer.position(info.offset);
            buffer.limit(info.offset + info.size);
            muxer.writeSampleData(track, buffer, info);
            written += info.size;
        }
        long now = System.currentTimeMillis();
        if (now - lastPublish > 900) {
            lastPublish = now;
            ScreenRecorder.publish(
                    "recording", target == null ? "" : target.toString(), size(), startedAt, "");
        }
    }

    private static boolean finishMuxer() {
        boolean ok = false;
        synchronized (EngineRecorder.class) {
            try {
                if (muxer != null && track >= 0) {
                    muxer.stop();
                    ok = true;
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
            track = -1;
        }
        return ok;
    }

    private static void closeQuietly(MediaCodec encoder, Surface input, MediaMuxer out) {
        if (out != null) {
            try {
                out.release();
            } catch (Throwable ignored) {
            }
        }
        if (encoder != null) {
            try {
                encoder.release();
            } catch (Throwable ignored) {
            }
        }
        if (input != null) {
            try {
                input.release();
            } catch (Throwable ignored) {
            }
        }
    }

    private static void openTarget(Context context) {
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
                        context.getContentResolver()
                                .insert(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, values);
                if (target != null) {
                    targetFile = context.getContentResolver().openFileDescriptor(target, "rw");
                }
            } catch (Throwable error) {
                target = null;
                targetFile = null;
            }
            if (targetFile != null) {
                return;
            }
        }
        File folder = ScreenRecorder.localRecordingsFolder(context);
        legacyTarget = new File(folder, name);
        try {
            targetFile =
                    ParcelFileDescriptor.open(
                            legacyTarget,
                            ParcelFileDescriptor.MODE_CREATE
                                    | ParcelFileDescriptor.MODE_READ_WRITE
                                    | ParcelFileDescriptor.MODE_TRUNCATE);
        } catch (Throwable error) {
            targetFile = null;
        }
    }

    private static File legacyFolder(Context context) {
        return RecorderService.legacyFolder(context);
    }

    /** Marks the entry finished so galleries can see it, and drops a file that never started. */
    private static void closeTarget() {
        try {
            if (targetFile != null) {
                targetFile.close();
            }
        } catch (Throwable ignored) {
        }
        targetFile = null;
        Context context = appContext;
        if (context == null) {
            return;
        }
        if (written == 0L) {
            if (target != null) {
                try {
                    context.getContentResolver().delete(target, null, null);
                } catch (Throwable ignored) {
                }
                target = null;
            }
            if (legacyTarget != null) {
                try {
                    legacyTarget.delete();
                } catch (Throwable ignored) {
                }
            }
        } else if (target != null) {
            try {
                ContentValues values = new ContentValues();
                values.put(MediaStore.Video.Media.IS_PENDING, 0);
                context.getContentResolver().update(target, values, null, null);
            } catch (Throwable ignored) {
            }
        }
    }

    private static void deleteTarget(Context context) {
        try {
            if (targetFile != null) {
                targetFile.close();
            }
        } catch (Throwable ignored) {
        }
        targetFile = null;
        if (target != null) {
            try {
                context.getContentResolver().delete(target, null, null);
            } catch (Throwable ignored) {
            }
            target = null;
        }
        if (legacyTarget != null) {
            try {
                legacyTarget.delete();
            } catch (Throwable ignored) {
            }
            legacyTarget = null;
        }
    }

    private static long size() {
        long size = written;
        try {
            if (targetFile != null) {
                size = Math.max(size, targetFile.getStatSize());
            }
        } catch (Throwable ignored) {
        }
        if (size <= 0 && legacyTarget != null) {
            size = legacyTarget.length();
        }
        return size;
    }

    /**
     * The largest size the display can be recorded at that a hardware AVC encoder accepts.
     * Answers {@code {width, height, fps}}, or {@code null}.
     */
    private static int[] fit(int wantedWidth, int wantedHeight, int wantedFps) {
        int[][] sizes = {
            {wantedWidth, wantedHeight},
            {1920, 1080},
            {1280, 720},
            {960, 540},
            {854, 480},
            {640, 360},
            {426, 240},
        };
        int[] rates = {wantedFps, 60, 30};
        for (int[] size : sizes) {
            int w = size[0] - (size[0] % 2);
            int h = size[1] - (size[1] % 2);
            if (w < 160 || h < 120) {
                continue;
            }
            for (int rate : rates) {
                if (rate <= 0) {
                    continue;
                }
                MediaFormat format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, w, h);
                format.setInteger(
                        MediaFormat.KEY_COLOR_FORMAT,
                        MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
                format.setInteger(MediaFormat.KEY_FRAME_RATE, rate);
                format.setInteger(MediaFormat.KEY_BIT_RATE, bitrate(w, h, rate));
                format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
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

    /** `yyyyMMdd-HHmmss`, the same name a screen recording gets. */
    private static String timestamp() {
        java.text.SimpleDateFormat format =
                new java.text.SimpleDateFormat("yyyyMMdd-HHmmss", java.util.Locale.US);
        return format.format(new java.util.Date());
    }

    private static int bitrate(int width, int height, int fps) {
        // Roughly 0.1 bits a pixel a frame, kept inside what phones encode comfortably.
        long wanted = (long) width * height * fps / 10L;
        return (int) Math.max(2_000_000L, Math.min(wanted, 24_000_000L));
    }
}
