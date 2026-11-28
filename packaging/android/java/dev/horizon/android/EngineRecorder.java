package dev.horizon.android;

import android.content.ContentValues;
import android.content.Context;
import android.media.AudioFormat;
import android.media.AudioRecord;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaFormat;
import android.media.MediaMuxer;
import android.media.MediaRecorder;
import android.net.Uri;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.provider.MediaStore;
import android.view.Surface;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
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
    private static File tempVideo;
    private static MicCapture mic;
    private static boolean externalAudio;
    private static String displayName;
    private static ParcelFileDescriptor targetFile;
    private static int track = -1;
    private static int width;
    private static int height;
    private static int frames;
    private static long startedAt;
    private static volatile long sessionStartUs;
    private static volatile long firstFrameQueuedUs;
    private static long firstVideoMonotonicUs = -1L;
    private static long firstRawPtsUs = Long.MIN_VALUE;
    private static long lastWrittenVideoPtsUs = -1L;
    private static volatile long stopRequestedAtMs;
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
    static synchronized String start(
            Context context, int wantedWidth, int wantedHeight, int wantedFps, boolean withMic) {
        return startInternal(context, wantedWidth, wantedHeight, wantedFps, withMic, false);
    }

    /** Starts raw video-only capture into a temporary MP4 for {@link RecorderService} to merge with game audio. */
    static synchronized String startTempOnly(
            Context context, int wantedWidth, int wantedHeight, int wantedFps) {
        return startInternal(context, wantedWidth, wantedHeight, wantedFps, false, true);
    }

    private static synchronized String startInternal(
            Context context,
            int wantedWidth,
            int wantedHeight,
            int wantedFps,
            boolean withMic,
            boolean extAudio) {
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
        externalAudio = extAudio;
        tempVideo = null;
        mic = null;
        target = null;
        legacyTarget = null;
        targetFile = null;
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
            sessionStartUs = System.nanoTime() / 1000L;
            firstFrameQueuedUs = 0L;
            firstVideoMonotonicUs = -1L;
            firstRawPtsUs = Long.MIN_VALUE;
            lastWrittenVideoPtsUs = -1L;
            stopRequestedAtMs = 0L;
            if (withMic || extAudio) {
                tempVideo = File.createTempFile("volt-raw-video-", ".mp4", appContext.getCacheDir());
                out = new MediaMuxer(
                        tempVideo.getAbsolutePath(), MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
                if (withMic) {
                    mic = new MicCapture(appContext.getCacheDir(), sessionStartUs);
                    mic.start();
                }
            } else {
                openTarget(appContext);
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
            }
            encoder.start();
        } catch (Throwable error) {
            if (mic != null) {
                try {
                    mic.stopAndWait();
                } catch (Throwable ignored) {
                }
                if (mic.file() != null) {
                    mic.file().delete();
                }
                mic = null;
            }
            if (tempVideo != null) {
                tempVideo.delete();
                tempVideo = null;
            }
            closeQuietly(encoder, input, out);
            deleteTarget(appContext);
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

    static void noteFrameQueued() {
        if (firstFrameQueuedUs == 0L) {
            firstFrameQueuedUs = System.nanoTime() / 1000L;
        }
    }

    static long sessionStartUs() {
        return sessionStartUs;
    }

    static synchronized long firstVideoMonotonicUs() {
        return firstVideoMonotonicUs;
    }

    static void beginStopInput() {
        MediaCodec encoder;
        synchronized (EngineRecorder.class) {
            if (!started) {
                return;
            }
            if (!stopping) {
                stopping = true;
                stopRequestedAtMs = System.currentTimeMillis();
                encoder = codec;
            } else {
                encoder = null;
            }
        }
        if (encoder != null) {
            try {
                encoder.signalEndOfInputStream();
            } catch (Throwable ignored) {
            }
        }
    }

    /** Stops capture synchronously (off the UI/game thread) and returns the recorded video file when {@code externalAudio} is set. */
    static File stopAndWaitTempFile() {
        beginStopInput();
        return awaitTempFile();
    }

    static File awaitTempFile() {
        synchronized (EngineRecorder.class) {
            if (!started) {
                return null;
            }
            stopping = true;
            if (stopRequestedAtMs == 0L) {
                stopRequestedAtMs = System.currentTimeMillis();
            }
        }
        stopCodecAndDrain();
        finishMuxer();
        File result = written > 0L ? tempVideo : null;
        if (result == null && tempVideo != null) {
            tempVideo.delete();
        }
        tempVideo = null;
        externalAudio = false;
        synchronized (EngineRecorder.class) {
            started = false;
            stopping = false;
        }
        return result;
    }

    private static void stopCodecAndDrain() {
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
            stopRequestedAtMs = System.currentTimeMillis();
        }
        Thread finisher = new Thread(EngineRecorder::finish, "Volt engine recorder stop");
        finisher.setDaemon(true);
        finisher.start();
        return "stopping;finishing the file";
    }

    /** Drains the last frames, closes the codec and finalises the MP4. Never on the game. */
    private static void finish() {
        MediaCodec encoder;
        synchronized (EngineRecorder.class) {
            encoder = codec;
        }
        if (encoder != null) {
            try {
                encoder.signalEndOfInputStream();
            } catch (Throwable ignored) {
            }
        }
        if (mic != null) {
            try {
                mic.stopAndWait();
            } catch (Throwable ignored) {
            }
        }
        stopCodecAndDrain();
        boolean finished = finishMuxer();
        boolean empty = written == 0L;
        long size = 0L;
        String uri = "";
        if (!empty && tempVideo != null && appContext != null) {
            openTarget(appContext);
            File micFile = mic == null ? null : mic.file();
            long audioShiftUs =
                    firstVideoMonotonicUs > 0L ? (sessionStartUs - firstVideoMonotonicUs) : 0L;
            if (targetFile != null) {
                boolean merged =
                        micFile != null
                                && micFile.exists()
                                && micFile.length() > 0
                                && RecorderService.Muxer.merge(
                                        tempVideo,
                                        micFile,
                                        targetFile.getFileDescriptor(),
                                        audioShiftUs);
                if (!merged) {
                    copyFileToFd(tempVideo, targetFile.getFileDescriptor());
                }
                size = size();
                Uri savedMedia = target;
                File savedLegacy = legacyTarget;
                String savedName = displayName;
                uri =
                        savedMedia != null
                                ? savedMedia.toString()
                                : (savedLegacy != null ? "file:" + savedLegacy.getAbsolutePath() : "");
                closeTarget();
                ScreenRecorder.mirrorToLocalRecordings(appContext, savedMedia, savedLegacy, savedName);
            }
        } else {
            size = size();
            Uri savedMedia = target;
            File savedLegacy = legacyTarget;
            String savedName = displayName;
            uri =
                    savedMedia != null
                            ? savedMedia.toString()
                            : (savedLegacy != null ? "file:" + savedLegacy.getAbsolutePath() : "");
            closeTarget();
            if (!empty && appContext != null) {
                ScreenRecorder.mirrorToLocalRecordings(appContext, savedMedia, savedLegacy, savedName);
            }
        }
        if (tempVideo != null) {
            tempVideo.delete();
            tempVideo = null;
        }
        if (mic != null && mic.file() != null) {
            mic.file().delete();
            mic = null;
        }
        externalAudio = false;
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

    private static void copyFileToFd(File from, java.io.FileDescriptor to) {
        if (from == null || to == null) {
            return;
        }
        try (FileInputStream in = new FileInputStream(from);
                FileOutputStream out = new FileOutputStream(to)) {
            byte[] buf = new byte[1 << 16];
            int n;
            while ((n = in.read(buf)) > 0) {
                out.write(buf, 0, n);
            }
            out.flush();
        } catch (Throwable ignored) {
        }
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
                    long stoppedAt = stopRequestedAtMs;
                    if (stoppedAt > 0L && System.currentTimeMillis() - stoppedAt > 2_000L) {
                        break;
                    }
                }
            }
        } catch (Throwable error) {
            ScreenRecorder.publish("error", "", 0L, 0L, ScreenRecorder.shorter(error));
        }
    }

    private static void startMuxer(MediaCodec encoder) {
        synchronized (EngineRecorder.class) {
            if (track >= 0 || muxer == null) {
                return;
            }
            track = muxer.addTrack(encoder.getOutputFormat());
            muxer.start();
        }
    }

    private static void write(ByteBuffer buffer, MediaCodec.BufferInfo info) {
        synchronized (EngineRecorder.class) {
            if (track < 0 || muxer == null) {
                return;
            }
            if (firstVideoMonotonicUs < 0L) {
                long nowUs = System.nanoTime() / 1000L;
                long rawPts = info.presentationTimeUs;
                boolean monotonic =
                        nowUs > 10_000_000L
                                && rawPts > 10_000_000L
                                && Math.abs(nowUs - rawPts) < 10_000_000L;
                firstRawPtsUs = rawPts;
                if (monotonic) {
                    firstVideoMonotonicUs = rawPts;
                } else {
                    long queuedUs = firstFrameQueuedUs;
                    firstVideoMonotonicUs = queuedUs > 0L ? queuedUs : nowUs;
                }
            }
            long pts = Math.max(0L, info.presentationTimeUs - firstRawPtsUs);
            if (lastWrittenVideoPtsUs >= 0L && pts <= lastWrittenVideoPtsUs) {
                pts = lastWrittenVideoPtsUs + 1L;
            }
            lastWrittenVideoPtsUs = pts;
            MediaCodec.BufferInfo outInfo = new MediaCodec.BufferInfo();
            outInfo.set(info.offset, info.size, pts, info.flags);
            buffer.position(info.offset);
            buffer.limit(info.offset + info.size);
            muxer.writeSampleData(track, buffer, outInfo);
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
     * The largest size the display can be recorded at that an AVC encoder accepts.
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
        MediaCodecInfo[] infos = null;
        try {
            infos = new MediaCodecList(MediaCodecList.REGULAR_CODECS).getCodecInfos();
        } catch (Throwable ignored) {
        }
        for (int[] size : sizes) {
            int w16 = Math.max(160, (size[0] + 8) & ~15);
            int h16 = Math.max(120, (size[1] + 8) & ~15);
            int w2 = Math.max(160, size[0] & ~1);
            int h2 = Math.max(120, size[1] & ~1);
            int[][] alignedCandidates = {{w16, h16}, {w2, h2}};
            for (int[] wh : alignedCandidates) {
                int w = wh[0];
                int h = wh[1];
                for (int rate : rates) {
                    if (rate <= 0) {
                        continue;
                    }
                    if (avcSupports(infos, w, h, rate)) {
                        return new int[] {w, h, rate};
                    }
                }
            }
        }
        // Final fallback: verify an AVC encoder exists and hand back 720p30 / 480p30.
        try {
            MediaCodec test = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC);
            if (test != null) {
                test.release();
                int w = Math.max(160, Math.min(1280, (wantedWidth + 8) & ~15));
                int h = Math.max(120, Math.min(720, (wantedHeight + 8) & ~15));
                return new int[] {w, h, Math.max(30, Math.min(60, wantedFps))};
            }
        } catch (Throwable ignored) {
        }
        return null;
    }

    private static boolean avcSupports(MediaCodecInfo[] infos, int w, int h, int fps) {
        if (infos == null) {
            return false;
        }
        for (MediaCodecInfo info : infos) {
            if (!info.isEncoder()) {
                continue;
            }
            for (String type : info.getSupportedTypes()) {
                if (!MediaFormat.MIMETYPE_VIDEO_AVC.equalsIgnoreCase(type)) {
                    continue;
                }
                try {
                    MediaCodecInfo.CodecCapabilities caps = info.getCapabilitiesForType(type);
                    MediaCodecInfo.VideoCapabilities vc = caps == null ? null : caps.getVideoCapabilities();
                    if (vc != null && vc.areSizeAndRateSupported(w, h, fps)) {
                        return true;
                    }
                } catch (Throwable ignored) {
                }
            }
        }
        return false;
    }

    /** Microphone AAC capture for raw gameplay recordings when Microphone audio is chosen. */
    private static final class MicCapture {
        private final File file;
        private final long originUs;
        private final int sampleRate = 44_100;
        private AudioRecord record;
        private MediaCodec codec;
        private Thread thread;
        private volatile boolean running;

        MicCapture(File cacheDir, long originUs) throws Exception {
            this.file = File.createTempFile("volt-mic-", ".mp4", cacheDir);
            this.originUs = originUs;
        }

        File file() {
            return file;
        }

        void start() throws Exception {
            int minimum =
                    AudioRecord.getMinBufferSize(
                            sampleRate, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT);
            int buffer = Math.max(minimum * 2, 16_384);
            record =
                    new AudioRecord(
                            MediaRecorder.AudioSource.MIC,
                            sampleRate,
                            AudioFormat.CHANNEL_IN_MONO,
                            AudioFormat.ENCODING_PCM_16BIT,
                            buffer);
            if (record.getState() != AudioRecord.STATE_INITIALIZED) {
                throw new IllegalStateException("microphone could not be opened");
            }
            MediaFormat format =
                    MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_AAC, sampleRate, 1);
            format.setInteger(
                    MediaFormat.KEY_AAC_PROFILE,
                    MediaCodecInfo.CodecProfileLevel.AACObjectLC);
            format.setInteger(MediaFormat.KEY_BIT_RATE, 96_000);
            format.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 16_384);
            codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_AUDIO_AAC);
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            codec.start();
            record.startRecording();
            running = true;
            thread = new Thread(this::pump, "Volt raw mic audio");
            thread.setDaemon(true);
            thread.start();
        }

        private void pump() {
            RecorderService.AudioCapture.pumpPcmToMp4(
                    record,
                    codec,
                    file,
                    sampleRate,
                    originUs,
                    () -> running);
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
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
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
