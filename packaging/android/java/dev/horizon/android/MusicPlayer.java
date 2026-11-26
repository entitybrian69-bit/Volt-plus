package dev.horizon.android;

import android.media.AudioAttributes;
import android.media.MediaDataSource;
import android.media.MediaPlayer;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.RandomAccessFile;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;

/**
 * Hardware-accelerated background music player for Volt+.
 * Runs on its own looper thread so playback, redirect/mirror resolution, and
 * progressive stream buffering never block the Vulkan game render thread.
 */
final class MusicPlayer {
    private static final MusicPlayer INSTANCE = new MusicPlayer();
    static MusicPlayer get() { return INSTANCE; }

    private static final String STREAM_USER_AGENT =
            "Mozilla/5.0 (Linux; Android 14; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
    private static final long PREPARE_TIMEOUT_MS = 15_000L;
    private static final int INITIAL_PREBUFFER_BYTES = 32 * 1024;

    private final HandlerThread thread = new HandlerThread("Volt music player");
    private final Handler handler;
    private final Object lock = new Object();

    private File cacheDir;
    private MediaPlayer player;
    private StreamingDataSource activeDataSource;
    private Runnable activeWatchdog;
    private int generation;
    private boolean playing;
    private boolean buffering;
    private boolean muted;
    private boolean pausedByLifecycle;
    private float volume = 0.85f;
    private long lastPositionMs;
    private long lastPositionAtMs;
    private long durationMs;

    private String[] trackIds = new String[0];
    private String[] urls = new String[0];
    private String[] fallbackLists = new String[0];
    private String[] titles = new String[0];
    private String[] artists = new String[0];
    private long[] durationsMs = new long[0];
    private int currentIndex;
    private String activePlaylist = "";
    private String status = "";

    private long audiusHostsFetchedAtMs;
    private List<String> cachedAudiusHosts = new ArrayList<>();

    private final Runnable positionTicker = new Runnable() {
        @Override
        public void run() {
            MediaPlayer mp;
            int gen;
            boolean shouldTick;
            synchronized (lock) {
                mp = player;
                gen = generation;
                shouldTick = playing && mp != null;
            }
            if (!shouldTick) {
                return;
            }
            try {
                if (mp.isPlaying()) {
                    int pos = mp.getCurrentPosition();
                    int dur = mp.getDuration();
                    synchronized (lock) {
                        if (gen == generation && player == mp) {
                            if (pos >= 0) {
                                lastPositionMs = pos;
                                lastPositionAtMs = SystemClock.elapsedRealtime();
                            }
                            if (dur > 0) {
                                durationMs = dur;
                            }
                        }
                    }
                }
            } catch (Throwable ignored) {}
            synchronized (lock) {
                if (gen == generation && playing && player != null) {
                    handler.postDelayed(this, 400L);
                }
            }
        }
    };

    private MusicPlayer() {
        thread.start();
        handler = new Handler(thread.getLooper());
    }

    void setCacheDir(File dir) {
        synchronized (lock) {
            if (dir != null) {
                cacheDir = dir;
            }
        }
    }

    private void scheduleTickerLocked() {
        handler.removeCallbacks(positionTicker);
        if (playing && player != null && !buffering) {
            handler.postDelayed(positionTicker, 400L);
        }
    }

    private void cancelWatchdogLocked() {
        if (activeWatchdog != null) {
            handler.removeCallbacks(activeWatchdog);
            activeWatchdog = null;
        }
    }

    private static String upgradeHttps(String url) {
        if (url == null) return "";
        String t = url.trim();
        if (t.startsWith("http://")) {
            return "https://" + t.substring("http://".length());
        }
        return t;
    }

    private static boolean isValidAudioContentType(String ct) {
        if (ct == null || ct.trim().isEmpty()) {
            return true;
        }
        String low = ct.trim().toLowerCase(Locale.ROOT);
        return !low.contains("text/html")
                && !low.contains("application/json")
                && !low.contains("application/xml")
                && !low.contains("text/plain")
                && !low.contains("mpegurl");
    }

    private static String readSmallString(HttpURLConnection conn) throws IOException {
        try (InputStream is = conn.getInputStream();
             ByteArrayOutputStream baos = new ByteArrayOutputStream()) {
            byte[] buf = new byte[4096];
            int total = 0;
            int n;
            while ((n = is.read(buf)) != -1) {
                baos.write(buf, 0, n);
                total += n;
                if (total > 512 * 1024) {
                    break;
                }
            }
            return baos.toString(StandardCharsets.UTF_8.name());
        }
    }

    private static String httpGetJsonString(String urlStr) {
        String current = upgradeHttps(urlStr);
        for (int hop = 0; hop < 5; hop++) {
            HttpURLConnection conn = null;
            try {
                conn = (HttpURLConnection) new URL(current).openConnection();
                conn.setInstanceFollowRedirects(false);
                conn.setRequestMethod("GET");
                conn.setConnectTimeout(6000);
                conn.setReadTimeout(8000);
                conn.setRequestProperty("User-Agent", STREAM_USER_AGENT);
                conn.setRequestProperty("Accept", "application/json");
                int code = conn.getResponseCode();
                if (code == 301 || code == 302 || code == 303 || code == 307 || code == 308) {
                    String loc = conn.getHeaderField("Location");
                    if (loc != null && !loc.trim().isEmpty()) {
                        current = upgradeHttps(new URL(new URL(current), loc.trim()).toString());
                        continue;
                    }
                }
                if (code >= 200 && code < 300) {
                    return readSmallString(conn);
                }
                return null;
            } catch (Throwable ignored) {
                return null;
            } finally {
                if (conn != null) {
                    try { conn.disconnect(); } catch (Throwable ignored) {}
                }
            }
        }
        return null;
    }

    private List<String> getAudiusHosts() {
        long now = SystemClock.elapsedRealtime();
        synchronized (lock) {
            if (!cachedAudiusHosts.isEmpty() && now - audiusHostsFetchedAtMs < 300_000L) {
                return new ArrayList<>(cachedAudiusHosts);
            }
        }
        List<String> hosts = new ArrayList<>();
        String body = httpGetJsonString("https://api.audius.co");
        if (body != null) {
            try {
                JSONObject json = new JSONObject(body);
                JSONArray arr = json.optJSONArray("data");
                if (arr != null) {
                    for (int i = 0; i < arr.length() && hosts.size() < 6; i++) {
                        String u = upgradeHttps(arr.optString(i, ""));
                        if (u.startsWith("https://")) {
                            while (u.endsWith("/")) {
                                u = u.substring(0, u.length() - 1);
                            }
                            if (!hosts.contains(u)) {
                                hosts.add(u);
                            }
                        }
                    }
                }
            } catch (Throwable ignored) {}
        }
        if (!hosts.contains("https://api.audius.co")) {
            hosts.add("https://api.audius.co");
        }
        synchronized (lock) {
            cachedAudiusHosts = new ArrayList<>(hosts);
            audiusHostsFetchedAtMs = SystemClock.elapsedRealtime();
        }
        return hosts;
    }

    private static String extractAudiusTrackId(String trackId, String primaryUrl) {
        if (trackId != null && trackId.startsWith("audius:") && trackId.length() > "audius:".length()) {
            return trackId.substring("audius:".length()).trim();
        }
        if (primaryUrl != null) {
            int idx = primaryUrl.indexOf("/v1/tracks/");
            if (idx >= 0) {
                String rest = primaryUrl.substring(idx + "/v1/tracks/".length());
                int end = rest.length();
                int slash = rest.indexOf('/');
                int q = rest.indexOf('?');
                if (slash >= 0) end = Math.min(end, slash);
                if (q >= 0) end = Math.min(end, q);
                String tid = rest.substring(0, end).trim();
                if (!tid.isEmpty()) {
                    return tid;
                }
            }
        }
        return null;
    }

    private static void addCandidate(LinkedHashSet<String> out, String raw) {
        if (raw == null) return;
        String u = upgradeHttps(raw);
        if (u.startsWith("https://")) {
            out.add(u);
        }
    }

    private List<String> buildCandidateUrls(String trackId, String primaryUrl, String fallbackList) {
        LinkedHashSet<String> candidates = new LinkedHashSet<>();
        addCandidate(candidates, primaryUrl);
        if (fallbackList != null && !fallbackList.isEmpty()) {
            for (String line : fallbackList.split("\n")) {
                addCandidate(candidates, line);
            }
        }
        String audiusId = extractAudiusTrackId(trackId, primaryUrl);
        if (audiusId != null) {
            List<String> hosts = getAudiusHosts();
            int limit = Math.min(5, hosts.size());
            for (int i = 0; i < limit; i++) {
                String host = hosts.get(i);
                String nrBody = httpGetJsonString(host + "/v1/tracks/" + audiusId + "/stream?no_redirect=true&app_name=VoltPlus");
                if (nrBody != null) {
                    try {
                        JSONObject json = new JSONObject(nrBody);
                        addCandidate(candidates, json.optString("data", ""));
                    } catch (Throwable ignored) {}
                }
                String metaBody = httpGetJsonString(host + "/v1/tracks/" + audiusId + "?app_name=VoltPlus");
                if (metaBody != null) {
                    try {
                        JSONObject json = new JSONObject(metaBody);
                        JSONObject data = json.optJSONObject("data");
                        if (data != null) {
                            JSONObject stream = data.optJSONObject("stream");
                            if (stream != null) {
                                addCandidate(candidates, stream.optString("url", ""));
                                JSONArray mirrors = stream.optJSONArray("mirrors");
                                if (mirrors != null) {
                                    for (int m = 0; m < mirrors.length(); m++) {
                                        addCandidate(candidates, mirrors.optString(m, ""));
                                    }
                                }
                            }
                        }
                    } catch (Throwable ignored) {}
                }
                addCandidate(candidates, host + "/v1/tracks/" + audiusId + "/stream?app_name=VoltPlus");
            }
        }
        return new ArrayList<>(candidates);
    }

    private static final class OpenedStream {
        final HttpURLConnection conn;
        final InputStream input;
        final long contentLength;

        OpenedStream(HttpURLConnection conn, InputStream input, long contentLength) {
            this.conn = conn;
            this.input = input;
            this.contentLength = contentLength;
        }
    }

    private static OpenedStream openAudioConnection(String rawUrl, int gen, MusicPlayer owner) throws IOException {
        String current = upgradeHttps(rawUrl);
        for (int hop = 0; hop < 10; hop++) {
            synchronized (owner.lock) {
                if (gen != owner.generation) {
                    throw new IOException("Cancelled");
                }
            }
            HttpURLConnection conn = (HttpURLConnection) new URL(current).openConnection();
            conn.setInstanceFollowRedirects(false);
            conn.setRequestMethod("GET");
            conn.setConnectTimeout(8000);
            conn.setReadTimeout(15000);
            conn.setRequestProperty("User-Agent", STREAM_USER_AGENT);
            conn.setRequestProperty("Accept", "audio/mpeg, audio/*;q=0.9, */*;q=0.8");
            int code;
            try {
                code = conn.getResponseCode();
            } catch (IOException ioe) {
                try { conn.disconnect(); } catch (Throwable ignored) {}
                throw ioe;
            }
            if (code == 301 || code == 302 || code == 303 || code == 307 || code == 308) {
                String loc = conn.getHeaderField("Location");
                try { conn.disconnect(); } catch (Throwable ignored) {}
                if (loc == null || loc.trim().isEmpty()) {
                    throw new IOException("Redirect HTTP " + code);
                }
                current = upgradeHttps(new URL(new URL(current), loc.trim()).toString());
                continue;
            }
            if (code < 200 || code >= 300) {
                try { conn.disconnect(); } catch (Throwable ignored) {}
                throw new IOException("HTTP " + code);
            }
            String ct = conn.getContentType();
            if (!isValidAudioContentType(ct)) {
                try { conn.disconnect(); } catch (Throwable ignored) {}
                throw new IOException("Invalid content-type: " + ct);
            }
            long len = conn.getContentLengthLong();
            InputStream is = conn.getInputStream();
            return new OpenedStream(conn, is, len);
        }
        throw new IOException("Too many redirects");
    }

    private static final class StreamingDataSource extends MediaDataSource {
        private final Object waitLock = new Object();
        private final Object fileReadLock = new Object();
        private final File cacheFile;
        private final HttpURLConnection conn;
        private final InputStream input;
        private final long expectedLength;
        private RandomAccessFile reader;
        private Thread pumpThread;
        private volatile long bytesAvailable;
        private volatile boolean eof;
        private volatile boolean closed;
        private volatile IOException streamError;

        StreamingDataSource(File cacheFile, OpenedStream opened) throws IOException {
            this.cacheFile = cacheFile;
            this.conn = opened.conn;
            this.input = opened.input;
            this.expectedLength = opened.contentLength > 0 ? opened.contentLength : -1L;
            this.reader = new RandomAccessFile(cacheFile, "r");
        }

        void startAndPrebuffer() throws IOException {
            final FileOutputStream fos = new FileOutputStream(cacheFile, false);
            byte[] initialBuf = new byte[8192];
            long written = 0L;
            try {
                while (written < INITIAL_PREBUFFER_BYTES && !closed) {
                    int n = input.read(initialBuf);
                    if (n == -1) {
                        eof = true;
                        break;
                    }
                    if (n > 0) {
                        fos.write(initialBuf, 0, n);
                        written += n;
                        bytesAvailable = written;
                    }
                }
                fos.flush();
            } catch (IOException ioe) {
                try { fos.close(); } catch (Throwable ignored) {}
                close();
                throw ioe;
            }
            if (written == 0L) {
                try { fos.close(); } catch (Throwable ignored) {}
                close();
                throw new IOException("Empty audio stream");
            }
            if (eof) {
                try { fos.close(); } catch (Throwable ignored) {}
                try { input.close(); } catch (Throwable ignored) {}
                try { conn.disconnect(); } catch (Throwable ignored) {}
                return;
            }

            Thread t = new Thread(() -> {
                byte[] buf = new byte[16 * 1024];
                try {
                    while (!closed) {
                        int n = input.read(buf);
                        if (n == -1) {
                            fos.flush();
                            synchronized (waitLock) {
                                eof = true;
                                waitLock.notifyAll();
                            }
                            break;
                        }
                        if (n > 0) {
                            fos.write(buf, 0, n);
                            fos.flush();
                            synchronized (waitLock) {
                                bytesAvailable += n;
                                waitLock.notifyAll();
                            }
                        }
                    }
                } catch (IOException ioe) {
                    synchronized (waitLock) {
                        streamError = ioe;
                        eof = true;
                        waitLock.notifyAll();
                    }
                } finally {
                    try { fos.close(); } catch (Throwable ignored) {}
                    try { input.close(); } catch (Throwable ignored) {}
                    try { conn.disconnect(); } catch (Throwable ignored) {}
                }
            }, "Volt music stream");
            t.setDaemon(true);
            pumpThread = t;
            t.start();
        }

        @Override
        public int readAt(long position, byte[] buffer, int offset, int size) throws IOException {
            if (size == 0) return 0;
            if (position < 0) return -1;
            synchronized (waitLock) {
                while (!closed && position >= bytesAvailable && !eof) {
                    try {
                        waitLock.wait(200L);
                    } catch (InterruptedException ie) {
                        Thread.currentThread().interrupt();
                        return -1;
                    }
                }
                if (closed) {
                    return -1;
                }
                if (position >= bytesAvailable && eof) {
                    if (streamError != null && bytesAvailable == 0) {
                        throw streamError;
                    }
                    return -1;
                }
            }
            synchronized (fileReadLock) {
                if (closed || reader == null) {
                    return -1;
                }
                long avail = bytesAvailable - position;
                if (avail <= 0) {
                    return -1;
                }
                int toRead = (int) Math.min(size, avail);
                reader.seek(position);
                int n = reader.read(buffer, offset, toRead);
                return n <= 0 ? -1 : n;
            }
        }

        @Override
        public long getSize() {
            if (expectedLength > 0) {
                return expectedLength;
            }
            if (eof) {
                return bytesAvailable;
            }
            return -1L;
        }

        @Override
        public void close() {
            synchronized (waitLock) {
                if (closed) return;
                closed = true;
                waitLock.notifyAll();
            }
            try { input.close(); } catch (Throwable ignored) {}
            try { conn.disconnect(); } catch (Throwable ignored) {}
            synchronized (fileReadLock) {
                if (reader != null) {
                    try { reader.close(); } catch (Throwable ignored) {}
                    reader = null;
                }
            }
            if (pumpThread != null) {
                pumpThread.interrupt();
            }
            try { cacheFile.delete(); } catch (Throwable ignored) {}
        }
    }

    void playQueue(
            String[] qIds,
            String[] qUrls,
            String[] qFallbacks,
            String[] qTitles,
            String[] qArtists,
            long[] qDurationsMs,
            int startIndex,
            String playlistName) {
        if (qUrls == null || qUrls.length == 0) {
            stop();
            return;
        }
        final int n = qUrls.length;
        final String[] safeIds = new String[n];
        final String[] safeUrls = new String[n];
        final String[] safeFallbacks = new String[n];
        final String[] safeTitles = new String[n];
        final String[] safeArtists = new String[n];
        final long[] safeDurs = new long[n];
        for (int i = 0; i < n; i++) {
            safeIds[i] = (qIds != null && i < qIds.length && qIds[i] != null) ? qIds[i] : "";
            safeUrls[i] = qUrls[i] != null ? qUrls[i] : "";
            safeFallbacks[i] = (qFallbacks != null && i < qFallbacks.length && qFallbacks[i] != null) ? qFallbacks[i] : "";
            safeTitles[i] = (qTitles != null && i < qTitles.length && qTitles[i] != null) ? qTitles[i] : "Unknown Track";
            safeArtists[i] = (qArtists != null && i < qArtists.length && qArtists[i] != null) ? qArtists[i] : "Unknown Artist";
            safeDurs[i] = (qDurationsMs != null && i < qDurationsMs.length) ? Math.max(0L, qDurationsMs[i]) : 0L;
        }
        final int idx = Math.max(0, Math.min(startIndex, n - 1));
        final String plist = playlistName != null ? playlistName : "";

        synchronized (lock) {
            trackIds = safeIds;
            urls = safeUrls;
            fallbackLists = safeFallbacks;
            titles = safeTitles;
            artists = safeArtists;
            durationsMs = safeDurs;
            currentIndex = idx;
            activePlaylist = plist;
            pausedByLifecycle = false;
        }
        startTrackAt(idx);
    }

    private void startTrackAt(int idx) {
        final int gen;
        final String trackId;
        final String url;
        final String fallbackList;
        final long fallbackDur;
        final File dir;
        synchronized (lock) {
            if (urls.length == 0) {
                return;
            }
            currentIndex = ((idx % urls.length) + urls.length) % urls.length;
            trackId = trackIds.length > currentIndex ? trackIds[currentIndex] : "";
            url = urls[currentIndex];
            fallbackList = fallbackLists.length > currentIndex ? fallbackLists[currentIndex] : "";
            fallbackDur = durationsMs[currentIndex];
            gen = ++generation;
            buffering = true;
            playing = true;
            lastPositionMs = 0L;
            lastPositionAtMs = SystemClock.elapsedRealtime();
            durationMs = fallbackDur;
            status = (url.startsWith("/") || url.startsWith("file://"))
                    ? "Loading offline track..."
                    : "Connecting to audio stream...";
            dir = cacheDir;
            handler.removeCallbacks(positionTicker);
            cancelWatchdogLocked();
        }

        handler.post(() -> {
            releasePlayerOnThread();
            synchronized (lock) {
                if (gen != generation) return;
            }
            if (url.isEmpty()) {
                synchronized (lock) {
                    if (gen == generation) {
                        buffering = false;
                        playing = false;
                        status = "Invalid track URL";
                    }
                }
                return;
            }
            try {
                MediaPlayer mp = new MediaPlayer();
                mp.setAudioAttributes(new AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build());

                StreamingDataSource dataSource = null;
                if (url.startsWith("/") || url.startsWith("file://")) {
                    String path = url.startsWith("file://") ? url.substring(7) : url;
                    File f = new File(path);
                    try (FileInputStream fis = new FileInputStream(f)) {
                        mp.setDataSource(fis.getFD());
                    }
                } else {
                    List<String> candidates = buildCandidateUrls(trackId, url, fallbackList);
                    synchronized (lock) {
                        if (gen != generation) {
                            try { mp.release(); } catch (Throwable ignored) {}
                            return;
                        }
                    }
                    String lastErr = "No reachable audio mirror";
                    File streamDir = (dir != null && dir.isDirectory())
                            ? dir
                            : new File(System.getProperty("java.io.tmpdir", "/data/local/tmp"));
                    for (String candidate : candidates) {
                        synchronized (lock) {
                            if (gen != generation) {
                                try { mp.release(); } catch (Throwable ignored) {}
                                return;
                            }
                            status = "Buffering audio stream...";
                        }
                        File tmpFile = File.createTempFile("volt_stream_" + gen + "_", ".bin", streamDir);
                        try {
                            OpenedStream opened = openAudioConnection(candidate, gen, this);
                            StreamingDataSource sds = new StreamingDataSource(tmpFile, opened);
                            sds.startAndPrebuffer();
                            dataSource = sds;
                            break;
                        } catch (Throwable t) {
                            try { tmpFile.delete(); } catch (Throwable ignored) {}
                            String msg = t.getMessage();
                            lastErr = (msg != null && !msg.isEmpty()) ? msg : t.getClass().getSimpleName();
                        }
                    }
                    if (dataSource == null) {
                        try { mp.release(); } catch (Throwable ignored) {}
                        synchronized (lock) {
                            if (gen == generation) {
                                buffering = false;
                                playing = false;
                                status = "Audio unavailable (" + lastErr + ")";
                                handler.removeCallbacks(positionTicker);
                            }
                        }
                        return;
                    }
                    mp.setDataSource(dataSource);
                }

                final StreamingDataSource boundSource = dataSource;
                mp.setOnPreparedListener(prepared -> {
                    boolean shouldStart;
                    float vol;
                    synchronized (lock) {
                        if (gen != generation || player != prepared) {
                            return;
                        }
                        cancelWatchdogLocked();
                        buffering = false;
                        int dur = prepared.getDuration();
                        if (dur > 0) {
                            durationMs = dur;
                        }
                        lastPositionMs = 0L;
                        lastPositionAtMs = SystemClock.elapsedRealtime();
                        vol = muted ? 0f : volume;
                        shouldStart = playing;
                        status = "";
                    }
                    try {
                        prepared.setVolume(vol, vol);
                        if (shouldStart) {
                            prepared.start();
                            synchronized (lock) {
                                scheduleTickerLocked();
                            }
                        }
                    } catch (Throwable ignored) {}
                });
                mp.setOnInfoListener((infoMp, what, extra) -> {
                    synchronized (lock) {
                        if (gen != generation || player != infoMp) {
                            return false;
                        }
                        if (what == MediaPlayer.MEDIA_INFO_BUFFERING_START) {
                            buffering = true;
                            lastPositionMs = estimatedPositionLocked();
                            lastPositionAtMs = SystemClock.elapsedRealtime();
                            status = "Buffering audio stream...";
                        } else if (what == MediaPlayer.MEDIA_INFO_BUFFERING_END) {
                            buffering = false;
                            lastPositionAtMs = SystemClock.elapsedRealtime();
                            status = "";
                            scheduleTickerLocked();
                        }
                    }
                    return false;
                });
                mp.setOnCompletionListener(finished -> {
                    synchronized (lock) {
                        if (gen != generation || player != finished) {
                            return;
                        }
                    }
                    next();
                });
                mp.setOnErrorListener((errMp, what, extra) -> {
                    synchronized (lock) {
                        if (gen == generation) {
                            cancelWatchdogLocked();
                            buffering = false;
                            playing = false;
                            status = "Playback error (" + what + "/" + extra + ")";
                            handler.removeCallbacks(positionTicker);
                        }
                    }
                    releasePlayerOnThread();
                    return true;
                });

                Runnable watchdog = () -> {
                    boolean timedOut;
                    synchronized (lock) {
                        timedOut = (gen == generation && player == mp && buffering);
                        if (timedOut) {
                            activeWatchdog = null;
                            buffering = false;
                            playing = false;
                            status = "Stream timed out — try another track";
                            handler.removeCallbacks(positionTicker);
                        }
                    }
                    if (timedOut) {
                        releasePlayerOnThread();
                    }
                };

                synchronized (lock) {
                    if (gen != generation) {
                        if (boundSource != null) {
                            try { boundSource.close(); } catch (Throwable ignored) {}
                        }
                        try { mp.release(); } catch (Throwable ignored) {}
                        return;
                    }
                    player = mp;
                    activeDataSource = boundSource;
                    activeWatchdog = watchdog;
                    handler.postDelayed(watchdog, PREPARE_TIMEOUT_MS);
                }
                mp.prepareAsync();
            } catch (Throwable e) {
                synchronized (lock) {
                    if (gen == generation) {
                        cancelWatchdogLocked();
                        buffering = false;
                        playing = false;
                        String msg = e.getMessage();
                        status = "Audio unavailable" + ((msg != null && !msg.isEmpty()) ? (": " + msg) : "");
                        handler.removeCallbacks(positionTicker);
                    }
                }
                releasePlayerOnThread();
            }
        });
    }

    void togglePause() {
        final int gen;
        final boolean targetPlay;
        synchronized (lock) {
            if (urls.length == 0) return;
            gen = generation;
            if (playing) {
                lastPositionMs = estimatedPositionLocked();
            }
            lastPositionAtMs = SystemClock.elapsedRealtime();
            playing = !playing;
            pausedByLifecycle = false;
            targetPlay = playing;
        }
        handler.post(() -> {
            MediaPlayer mp;
            synchronized (lock) {
                if (gen != generation || player == null) return;
                mp = player;
                if (buffering) return;
            }
            try {
                if (targetPlay) {
                    mp.start();
                    synchronized (lock) {
                        lastPositionAtMs = SystemClock.elapsedRealtime();
                        scheduleTickerLocked();
                    }
                } else if (mp.isPlaying()) {
                    int pos = mp.getCurrentPosition();
                    synchronized (lock) {
                        if (pos >= 0) lastPositionMs = pos;
                        lastPositionAtMs = SystemClock.elapsedRealtime();
                        handler.removeCallbacks(positionTicker);
                    }
                    mp.pause();
                }
            } catch (Throwable ignored) {}
        });
    }

    void next() {
        final int nextIdx;
        synchronized (lock) {
            if (urls.length == 0) return;
            nextIdx = (currentIndex + 1) % urls.length;
        }
        startTrackAt(nextIdx);
    }

    void prev() {
        final int targetIdx;
        final boolean restartCurrent;
        synchronized (lock) {
            if (urls.length == 0) return;
            long pos = estimatedPositionLocked();
            restartCurrent = pos > 3500L;
            targetIdx = restartCurrent ? currentIndex : (currentIndex - 1 + urls.length) % urls.length;
        }
        if (restartCurrent) {
            seekToMs(0L);
        } else {
            startTrackAt(targetIdx);
        }
    }

    void seekByMs(long deltaMs) {
        final int gen;
        synchronized (lock) {
            if (urls.length == 0) return;
            gen = generation;
        }
        handler.post(() -> {
            MediaPlayer mp;
            long dur;
            synchronized (lock) {
                if (gen != generation || player == null || buffering) return;
                mp = player;
                dur = durationMs;
            }
            try {
                long cur = Math.max(0L, mp.getCurrentPosition());
                long actualDur = mp.getDuration();
                if (actualDur > 0) dur = actualDur;
                long target = cur + deltaMs;
                if (target < 0L) target = 0L;
                if (dur > 0L && target > dur) target = Math.max(0L, dur - 500L);
                mp.seekTo((int) target);
                synchronized (lock) {
                    lastPositionMs = target;
                    lastPositionAtMs = SystemClock.elapsedRealtime();
                    if (actualDur > 0) durationMs = actualDur;
                }
            } catch (Throwable ignored) {}
        });
    }

    void seekToMs(long posMs) {
        final int gen;
        final long clamped = Math.max(0L, posMs);
        synchronized (lock) {
            if (urls.length == 0) return;
            gen = generation;
        }
        handler.post(() -> {
            MediaPlayer mp;
            synchronized (lock) {
                if (gen != generation || player == null || buffering) return;
                mp = player;
            }
            try {
                long dur = mp.getDuration();
                long target = dur > 0 ? Math.min(clamped, Math.max(0L, dur - 250L)) : clamped;
                mp.seekTo((int) target);
                synchronized (lock) {
                    lastPositionMs = target;
                    lastPositionAtMs = SystemClock.elapsedRealtime();
                }
            } catch (Throwable ignored) {}
        });
    }

    void setVolume(float vol) {
        final float clamped = Math.max(0f, Math.min(1f, vol));
        synchronized (lock) {
            volume = clamped;
        }
        applyGain();
    }

    void setMuted(boolean m) {
        synchronized (lock) {
            muted = m;
        }
        applyGain();
    }

    private void applyGain() {
        handler.post(() -> {
            MediaPlayer mp;
            float g;
            synchronized (lock) {
                mp = player;
                g = muted ? 0f : volume;
            }
            if (mp != null) {
                try { mp.setVolume(g, g); } catch (Throwable ignored) {}
            }
        });
    }

    void stop() {
        synchronized (lock) {
            ++generation;
            playing = false;
            buffering = false;
            pausedByLifecycle = false;
            lastPositionMs = 0L;
            lastPositionAtMs = SystemClock.elapsedRealtime();
            durationMs = 0L;
            trackIds = new String[0];
            urls = new String[0];
            fallbackLists = new String[0];
            titles = new String[0];
            artists = new String[0];
            durationsMs = new long[0];
            currentIndex = 0;
            status = "";
            handler.removeCallbacks(positionTicker);
            cancelWatchdogLocked();
        }
        handler.post(this::releasePlayerOnThread);
    }

    void onAppPause() {
        final int gen;
        synchronized (lock) {
            if (!playing) return;
            lastPositionMs = estimatedPositionLocked();
            lastPositionAtMs = SystemClock.elapsedRealtime();
            playing = false;
            pausedByLifecycle = true;
            gen = generation;
            handler.removeCallbacks(positionTicker);
        }
        handler.post(() -> {
            MediaPlayer mp;
            synchronized (lock) {
                if (gen != generation || player == null || buffering) return;
                mp = player;
            }
            try {
                if (mp.isPlaying()) {
                    int pos = mp.getCurrentPosition();
                    synchronized (lock) {
                        if (pos >= 0) lastPositionMs = pos;
                    }
                    mp.pause();
                }
            } catch (Throwable ignored) {}
        });
    }

    void onAppResume() {
        final int gen;
        synchronized (lock) {
            if (!pausedByLifecycle || urls.length == 0) return;
            pausedByLifecycle = false;
            playing = true;
            lastPositionAtMs = SystemClock.elapsedRealtime();
            gen = generation;
        }
        handler.post(() -> {
            MediaPlayer mp;
            synchronized (lock) {
                if (gen != generation || player == null || buffering) return;
                mp = player;
            }
            try {
                mp.start();
                synchronized (lock) {
                    lastPositionAtMs = SystemClock.elapsedRealtime();
                    scheduleTickerLocked();
                }
            } catch (Throwable ignored) {}
        });
    }

    private long estimatedPositionLocked() {
        if (!playing || buffering || player == null) {
            return lastPositionMs;
        }
        long elapsed = Math.max(0L, SystemClock.elapsedRealtime() - lastPositionAtMs);
        long pos = lastPositionMs + elapsed;
        if (durationMs > 0L && pos > durationMs) {
            pos = durationMs;
        }
        return pos;
    }

    String snapshot() {
        synchronized (lock) {
            long pos = estimatedPositionLocked();
            String title = (currentIndex >= 0 && currentIndex < titles.length) ? titles[currentIndex] : "";
            String artist = (currentIndex >= 0 && currentIndex < artists.length) ? artists[currentIndex] : "";
            String url = (currentIndex >= 0 && currentIndex < urls.length) ? urls[currentIndex] : "";
            return (playing ? "1" : "0") + "\t"
                    + (buffering ? "1" : "0") + "\t"
                    + (muted ? "1" : "0") + "\t"
                    + volume + "\t"
                    + pos + "\t"
                    + durationMs + "\t"
                    + currentIndex + "\t"
                    + urls.length + "\t"
                    + sanitizeField(activePlaylist) + "\t"
                    + sanitizeField(title) + "\t"
                    + sanitizeField(artist) + "\t"
                    + sanitizeField(url) + "\t"
                    + sanitizeField(status);
        }
    }

    private static String sanitizeField(String s) {
        if (s == null) return "";
        return s.replace('\t', ' ').replace('\n', ' ').replace('\r', ' ');
    }

    private void releasePlayerOnThread() {
        MediaPlayer mp;
        StreamingDataSource sds;
        synchronized (lock) {
            mp = player;
            player = null;
            sds = activeDataSource;
            activeDataSource = null;
            cancelWatchdogLocked();
        }
        if (sds != null) {
            try { sds.close(); } catch (Throwable ignored) {}
        }
        if (mp != null) {
            try { mp.stop(); } catch (Throwable ignored) {}
            try { mp.reset(); } catch (Throwable ignored) {}
            try { mp.release(); } catch (Throwable ignored) {}
        }
    }
}
