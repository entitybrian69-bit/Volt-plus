package dev.horizon.android;

import android.content.ContentValues;
import android.content.Context;
import android.media.Image;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaFormat;
import android.media.MediaMuxer;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.os.ParcelFileDescriptor;
import android.provider.MediaStore;
import java.io.File;
import java.nio.ByteBuffer;

/**
 * Deterministic offline H.264 MP4 encoder for Replay Mod camera path rendering.
 * Completely independent from {@link ScreenRecorder} and {@link EngineRecorder} so
 * live screen recording stays separate.
 */
public final class ReplayVideoEncoder {
    private static MediaCodec codec;
    private static MediaMuxer muxer;
    private static Thread drainThread;
    private static volatile boolean running;
    private static volatile boolean drainDone;
    private static volatile String drainError;
    private static int trackIndex = -1;
    private static int encWidth;
    private static int encHeight;
    private static int encFps;
    private static long writtenBytes;
    private static Context appContext;
    private static Uri mediaUri;
    private static File fallbackFile;
    private static ParcelFileDescriptor pfd;
    private static String outputLabel;

    private ReplayVideoEncoder() {}

    public static synchronized String start(
            Context context,
            int wantedWidth,
            int wantedHeight,
            int wantedFps,
            int bitrateMbps,
            String baseName) {
        return start(context, wantedWidth, wantedHeight, wantedFps, bitrateMbps, baseName, "");
    }

    public static synchronized String start(
            Context context,
            int wantedWidth,
            int wantedHeight,
            int wantedFps,
            int bitrateMbps,
            String baseName,
            String renderedDir) {
        if (running) {
            return "error;Replay video export already in progress";
        }
        appContext = context.getApplicationContext() != null ? context.getApplicationContext() : context;
        int w = Math.max(320, wantedWidth) & ~15;
        int h = Math.max(240, wantedHeight) & ~15;
        int fps = Math.max(15, Math.min(120, wantedFps));
        int bitrate = Math.max(4, Math.min(60, bitrateMbps)) * 1_000_000;

        int[] fitted = fitSize(w, h, fps);
        encWidth = fitted[0];
        encHeight = fitted[1];
        encFps = fitted[2];
        trackIndex = -1;
        writtenBytes = 0L;
        drainError = null;
        drainDone = false;
        mediaUri = null;
        fallbackFile = null;
        pfd = null;
        outputLabel = sanitizeName(baseName) + ".mp4";

        try {
            openOutput(appContext, outputLabel, renderedDir);
            if (pfd == null) {
                return "error;Could not open output MP4 destination";
            }

            MediaFormat format =
                    MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, encWidth, encHeight);
            format.setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420Flexible);
            format.setInteger(MediaFormat.KEY_BIT_RATE, bitrate);
            format.setInteger(MediaFormat.KEY_FRAME_RATE, encFps);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);

            codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC);
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            if (fallbackFile != null) {
                muxer = new MediaMuxer(fallbackFile.getAbsolutePath(), MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
            } else {
                muxer = new MediaMuxer(pfd.getFileDescriptor(), MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4);
            }
            codec.start();
            running = true;

            drainThread = new Thread(ReplayVideoEncoder::drainLoop, "Volt replay video drain");
            drainThread.setDaemon(true);
            drainThread.start();
            return "ok;" + encWidth + "x" + encHeight + "@" + encFps;
        } catch (Throwable t) {
            cleanupFailed();
            return "error;" + shortMsg(t);
        }
    }

    public static synchronized String pushFrame(
            byte[] rgba, int srcWidth, int srcHeight, long ptsUs) {
        if (!running || codec == null) {
            return "error;Encoder not running";
        }
        if (drainError != null) {
            return "error;" + drainError;
        }
        try {
            int inputIndex = codec.dequeueInputBuffer(250_000);
            if (inputIndex < 0) {
                return "retry;encoder busy";
            }
            Image image = codec.getInputImage(inputIndex);
            if (image != null) {
                fillYuv420FromRgba(image, rgba, srcWidth, srcHeight, encWidth, encHeight);
                codec.queueInputBuffer(inputIndex, 0, imageSizeEstimate(encWidth, encHeight), ptsUs, 0);
            } else {
                ByteBuffer buf = codec.getInputBuffer(inputIndex);
                if (buf == null) {
                    return "error;Encoder offered null input buffer";
                }
                int bytes = fillI420BufferFromRgba(buf, rgba, srcWidth, srcHeight, encWidth, encHeight);
                codec.queueInputBuffer(inputIndex, 0, bytes, ptsUs, 0);
            }
            return "ok";
        } catch (Throwable t) {
            return "error;" + shortMsg(t);
        }
    }

    public static synchronized String finish() {
        if (!running || codec == null) {
            return "error;Encoder not active";
        }
        try {
            int inputIndex = codec.dequeueInputBuffer(1_000_000);
            if (inputIndex >= 0) {
                codec.queueInputBuffer(inputIndex, 0, 0, 0L, MediaCodec.BUFFER_FLAG_END_OF_STREAM);
            }
        } catch (Throwable ignored) {
        }

        running = false;
        if (drainThread != null) {
            try {
                drainThread.join(10_000);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
            drainThread = null;
        }

        boolean hadOutput = writtenBytes > 0L && trackIndex >= 0;
        String err = drainError;
        releaseCodecAndMuxer(hadOutput);

        if (err != null) {
            deletePartial();
            return "error;" + err;
        }
        if (!hadOutput) {
            deletePartial();
            return "error;No video frames were written";
        }
        String savedWhere = finalizeOutput();
        return "saved;" + savedWhere;
    }

    public static synchronized void cancel() {
        running = false;
        if (drainThread != null) {
            drainThread.interrupt();
            try {
                drainThread.join(1_500);
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
            drainThread = null;
        }
        releaseCodecAndMuxer(false);
        deletePartial();
    }

    private static void drainLoop() {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        try {
            while (!drainDone) {
                MediaCodec c = codec;
                if (c == null) {
                    break;
                }
                int idx = c.dequeueOutputBuffer(info, 20_000);
                if (idx == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    if (muxer != null && trackIndex < 0) {
                        trackIndex = muxer.addTrack(c.getOutputFormat());
                        muxer.start();
                    }
                } else if (idx >= 0) {
                    ByteBuffer out = c.getOutputBuffer(idx);
                    if (out != null
                            && info.size > 0
                            && (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0
                            && muxer != null
                            && trackIndex >= 0) {
                        out.position(info.offset);
                        out.limit(info.offset + info.size);
                        muxer.writeSampleData(trackIndex, out, info);
                        writtenBytes += info.size;
                    }
                    c.releaseOutputBuffer(idx, false);
                    if ((info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0) {
                        drainDone = true;
                        break;
                    }
                } else if (!running && idx == MediaCodec.INFO_TRY_AGAIN_LATER) {
                    break;
                }
            }
        } catch (Throwable t) {
            drainError = shortMsg(t);
        }
    }

    private static void fillYuv420FromRgba(
            Image image, byte[] rgba, int srcW, int srcH, int dstW, int dstH) {
        Image.Plane[] planes = image.getPlanes();
        Image.Plane yPlane = planes[0];
        Image.Plane uPlane = planes[1];
        Image.Plane vPlane = planes[2];

        ByteBuffer yBuf = yPlane.getBuffer();
        ByteBuffer uBuf = uPlane.getBuffer();
        ByteBuffer vBuf = vPlane.getBuffer();
        int yRowStride = yPlane.getRowStride();
        int yPixStride = yPlane.getPixelStride();
        int uRowStride = uPlane.getRowStride();
        int uPixStride = uPlane.getPixelStride();
        int vRowStride = vPlane.getRowStride();
        int vPixStride = vPlane.getPixelStride();

        for (int dy = 0; dy < dstH; dy++) {
            int sy = (dy * srcH) / dstH;
            int yRowOff = dy * yRowStride;
            int uvRowOffU = (dy >> 1) * uRowStride;
            int uvRowOffV = (dy >> 1) * vRowStride;
            for (int dx = 0; dx < dstW; dx++) {
                int sx = (dx * srcW) / dstW;
                int p = (sy * srcW + sx) * 4;
                int r = rgba[p] & 0xFF;
                int g = rgba[p + 1] & 0xFF;
                int b = rgba[p + 2] & 0xFF;

                int yVal = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
                yBuf.put(yRowOff + dx * yPixStride, (byte) clampByte(yVal));

                if ((dy & 1) == 0 && (dx & 1) == 0) {
                    int uVal = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                    int vVal = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                    int uvCol = dx >> 1;
                    uBuf.put(uvRowOffU + uvCol * uPixStride, (byte) clampByte(uVal));
                    vBuf.put(uvRowOffV + uvCol * vPixStride, (byte) clampByte(vVal));
                }
            }
        }
    }

    private static int fillI420BufferFromRgba(
            ByteBuffer buf, byte[] rgba, int srcW, int srcH, int dstW, int dstH) {
        buf.clear();
        int ySize = dstW * dstH;
        int uvSize = (dstW / 2) * (dstH / 2);
        for (int dy = 0; dy < dstH; dy++) {
            int sy = (dy * srcH) / dstH;
            for (int dx = 0; dx < dstW; dx++) {
                int sx = (dx * srcW) / dstW;
                int p = (sy * srcW + sx) * 4;
                int r = rgba[p] & 0xFF;
                int g = rgba[p + 1] & 0xFF;
                int b = rgba[p + 2] & 0xFF;
                int yVal = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
                buf.put(dy * dstW + dx, (byte) clampByte(yVal));
                if ((dy & 1) == 0 && (dx & 1) == 0) {
                    int uVal = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                    int vVal = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                    int uvIdx = (dy >> 1) * (dstW / 2) + (dx >> 1);
                    buf.put(ySize + uvIdx, (byte) clampByte(uVal));
                    buf.put(ySize + uvSize + uvIdx, (byte) clampByte(vVal));
                }
            }
        }
        return ySize + uvSize * 2;
    }

    private static int imageSizeEstimate(int w, int h) {
        return (w * h * 3) / 2;
    }

    private static int clampByte(int v) {
        return v < 0 ? 0 : (Math.min(v, 255));
    }

    private static int[] fitSize(int w, int h, int fps) {
        int[][] candidates = {{w, h}, {1920, 1080}, {1280, 720}, {854, 480}};
        MediaCodecList list = new MediaCodecList(MediaCodecList.REGULAR_CODECS);
        for (MediaCodecInfo info : list.getCodecInfos()) {
            if (!info.isEncoder()) {
                continue;
            }
            try {
                MediaCodecInfo.CodecCapabilities caps =
                        info.getCapabilitiesForType(MediaFormat.MIMETYPE_VIDEO_AVC);
                MediaCodecInfo.VideoCapabilities vc = caps.getVideoCapabilities();
                if (vc == null) {
                    continue;
                }
                for (int[] c : candidates) {
                    int cw = c[0] & ~15;
                    int ch = c[1] & ~15;
                    if (vc.areSizeAndRateSupported(cw, ch, fps)) {
                        return new int[] {cw, ch, fps};
                    }
                    if (vc.areSizeAndRateSupported(cw, ch, 30)) {
                        return new int[] {cw, ch, 30};
                    }
                }
            } catch (Throwable ignored) {
            }
        }
        return new int[] {w, h, fps};
    }

    private static File uniqueLocalFile(File dir, String fileName) {
        String stem = fileName.endsWith(".mp4")
                ? fileName.substring(0, fileName.length() - 4)
                : fileName;
        File candidate = new File(dir, stem + ".mp4");
        if (!candidate.exists()) {
            return candidate;
        }
        for (int i = 2; i < 10_000; i++) {
            File next = new File(dir, stem + "_" + i + ".mp4");
            if (!next.exists()) {
                return next;
            }
        }
        return new File(dir, stem + "_" + System.currentTimeMillis() + ".mp4");
    }

    private static void openOutput(Context context, String fileName, String renderedDir)
            throws Exception {
        if (renderedDir != null && !renderedDir.isEmpty()) {
            File dir = new File(renderedDir);
            if (!dir.exists()) {
                dir.mkdirs();
            }
            if (dir.isDirectory()) {
                fallbackFile = uniqueLocalFile(dir, fileName);
                outputLabel = fallbackFile.getName();
                pfd =
                        ParcelFileDescriptor.open(
                                fallbackFile,
                                ParcelFileDescriptor.MODE_CREATE
                                        | ParcelFileDescriptor.MODE_READ_WRITE
                                        | ParcelFileDescriptor.MODE_TRUNCATE);
                return;
            }
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ContentValues values = new ContentValues();
            values.put(MediaStore.Video.Media.DISPLAY_NAME, fileName);
            values.put(MediaStore.Video.Media.MIME_TYPE, "video/mp4");
            values.put(
                    MediaStore.Video.Media.RELATIVE_PATH,
                    Environment.DIRECTORY_MOVIES + "/VoltPlus Replays");
            values.put(MediaStore.Video.Media.IS_PENDING, 1);
            mediaUri =
                    context.getContentResolver()
                            .insert(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, values);
            if (mediaUri != null) {
                pfd = context.getContentResolver().openFileDescriptor(mediaUri, "rw");
                if (pfd != null) {
                    return;
                }
            }
        }
        File dir = context.getExternalFilesDir(Environment.DIRECTORY_MOVIES);
        if (dir == null) {
            dir = new File(context.getFilesDir(), "replays_mp4");
        }
        if (!dir.exists()) {
            dir.mkdirs();
        }
        fallbackFile = uniqueLocalFile(dir, fileName);
        outputLabel = fallbackFile.getName();
        pfd =
                ParcelFileDescriptor.open(
                        fallbackFile,
                        ParcelFileDescriptor.MODE_CREATE
                                | ParcelFileDescriptor.MODE_READ_WRITE
                                | ParcelFileDescriptor.MODE_TRUNCATE);
    }

    private static String finalizeOutput() {
        try {
            if (pfd != null) {
                pfd.close();
            }
        } catch (Throwable ignored) {
        }
        pfd = null;
        if (fallbackFile != null && fallbackFile.exists() && appContext != null && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            try {
                ContentValues values = new ContentValues();
                values.put(MediaStore.Video.Media.DISPLAY_NAME, fallbackFile.getName());
                values.put(MediaStore.Video.Media.MIME_TYPE, "video/mp4");
                values.put(
                        MediaStore.Video.Media.RELATIVE_PATH,
                        Environment.DIRECTORY_MOVIES + "/VoltPlus Replays");
                values.put(MediaStore.Video.Media.IS_PENDING, 1);
                Uri exported =
                        appContext.getContentResolver()
                                .insert(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, values);
                if (exported != null) {
                    try (java.io.InputStream in = new java.io.FileInputStream(fallbackFile);
                            java.io.OutputStream out = appContext.getContentResolver().openOutputStream(exported)) {
                        if (out != null) {
                            byte[] buf = new byte[65536];
                            int n;
                            while ((n = in.read(buf)) >= 0) {
                                out.write(buf, 0, n);
                            }
                        }
                    }
                    ContentValues done = new ContentValues();
                    done.put(MediaStore.Video.Media.IS_PENDING, 0);
                    appContext.getContentResolver().update(exported, done, null, null);
                }
            } catch (Throwable ignored) {
            }
            return "replays/rendered/" + fallbackFile.getName() + " & Movies/VoltPlus Replays";
        }
        if (mediaUri != null && appContext != null) {
            try {
                ContentValues values = new ContentValues();
                values.put(MediaStore.Video.Media.IS_PENDING, 0);
                appContext.getContentResolver().update(mediaUri, values, null, null);
            } catch (Throwable ignored) {
            }
            return "Movies/VoltPlus Replays/" + outputLabel;
        }
        return fallbackFile != null ? fallbackFile.getAbsolutePath() : outputLabel;
    }

    public static String playDocument(android.app.Activity activity, String docId) {
        if (activity == null || docId == null || docId.isEmpty()) {
            return "error;nothing to play";
        }
        try {
            Uri uri =
                    android.provider.DocumentsContract.buildDocumentUri(
                            activity.getPackageName() + ".documents", docId);
            final android.content.Intent view =
                    new android.content.Intent(android.content.Intent.ACTION_VIEW);
            view.setDataAndType(uri, "video/mp4");
            view.addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.runOnUiThread(() -> {
                try {
                    activity.startActivity(view);
                } catch (Throwable ignored) {
                }
            });
            return "ok;Opening rendered replay...";
        } catch (Throwable error) {
            return "error;" + shortMsg(error);
        }
    }

    private static void deletePartial() {
        try {
            if (pfd != null) {
                pfd.close();
            }
        } catch (Throwable ignored) {
        }
        pfd = null;
        if (mediaUri != null && appContext != null) {
            try {
                appContext.getContentResolver().delete(mediaUri, null, null);
            } catch (Throwable ignored) {
            }
            mediaUri = null;
        }
        if (fallbackFile != null) {
            try {
                fallbackFile.delete();
            } catch (Throwable ignored) {
            }
            fallbackFile = null;
        }
    }

    private static void releaseCodecAndMuxer(boolean stopMuxer) {
        if (codec != null) {
            try {
                codec.stop();
            } catch (Throwable ignored) {
            }
            try {
                codec.release();
            } catch (Throwable ignored) {
            }
            codec = null;
        }
        if (muxer != null) {
            if (stopMuxer) {
                try {
                    muxer.stop();
                } catch (Throwable ignored) {
                }
            }
            try {
                muxer.release();
            } catch (Throwable ignored) {
            }
            muxer = null;
        }
    }

    private static void cleanupFailed() {
        running = false;
        releaseCodecAndMuxer(false);
        deletePartial();
    }

    private static String sanitizeName(String raw) {
        if (raw == null || raw.trim().isEmpty()) {
            return "VoltPlus_Replay_" + System.currentTimeMillis();
        }
        String clean = raw.trim().replaceAll("[^A-Za-z0-9._-]+", "_");
        return clean.isEmpty() ? ("VoltPlus_Replay_" + System.currentTimeMillis()) : clean;
    }

    private static String shortMsg(Throwable t) {
        String m = t.getMessage();
        return (m == null || m.isEmpty()) ? t.getClass().getSimpleName() : m;
    }
}
