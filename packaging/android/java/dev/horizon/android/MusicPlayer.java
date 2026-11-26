package dev.horizon.android;

import android.media.AudioAttributes;
import android.media.MediaPlayer;
import android.os.Handler;
import android.os.HandlerThread;

/**
 * Hardware-accelerated background music player for Volt+.
 * Runs on its own looper thread so playback and playlist auto-advance
 * continue seamlessly between the launcher and Minecraft gameplay.
 */
final class MusicPlayer {
    private static final MusicPlayer INSTANCE = new MusicPlayer();
    static MusicPlayer get() { return INSTANCE; }

    private final HandlerThread thread = new HandlerThread("Volt music player");
    private final Handler handler;
    private final Object lock = new Object();

    private MediaPlayer player;
    private int generation;
    private boolean playing;
    private boolean buffering;
    private boolean muted;
    private boolean pausedByLifecycle;
    private float volume = 0.85f;
    private long lastPositionMs;
    private long durationMs;

    private String[] urls = new String[0];
    private String[] titles = new String[0];
    private String[] artists = new String[0];
    private long[] durationsMs = new long[0];
    private int currentIndex;
    private String activePlaylist = "";
    private String status = "";

    private MusicPlayer() {
        thread.start();
        handler = new Handler(thread.getLooper());
    }

    void playQueue(
            String[] qUrls,
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
        final String[] safeUrls = new String[n];
        final String[] safeTitles = new String[n];
        final String[] safeArtists = new String[n];
        final long[] safeDurs = new long[n];
        for (int i = 0; i < n; i++) {
            safeUrls[i] = qUrls[i] != null ? qUrls[i] : "";
            safeTitles[i] = (qTitles != null && i < qTitles.length && qTitles[i] != null) ? qTitles[i] : "Unknown Track";
            safeArtists[i] = (qArtists != null && i < qArtists.length && qArtists[i] != null) ? qArtists[i] : "Unknown Artist";
            safeDurs[i] = (qDurationsMs != null && i < qDurationsMs.length) ? Math.max(0L, qDurationsMs[i]) : 0L;
        }
        final int idx = Math.max(0, Math.min(startIndex, n - 1));
        final String plist = playlistName != null ? playlistName : "";

        synchronized (lock) {
            urls = safeUrls;
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
        final String url;
        final long fallbackDur;
        synchronized (lock) {
            if (urls.length == 0) {
                return;
            }
            currentIndex = ((idx % urls.length) + urls.length) % urls.length;
            url = urls[currentIndex];
            fallbackDur = durationsMs[currentIndex];
            gen = ++generation;
            buffering = true;
            playing = true;
            lastPositionMs = 0L;
            durationMs = fallbackDur;
            status = "Loading track...";
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
                mp.setDataSource(url);
                mp.setOnPreparedListener(prepared -> {
                    boolean shouldStart;
                    float vol;
                    synchronized (lock) {
                        if (gen != generation || player != prepared) {
                            return;
                        }
                        buffering = false;
                        int dur = prepared.getDuration();
                        if (dur > 0) {
                            durationMs = dur;
                        }
                        vol = muted ? 0f : volume;
                        shouldStart = playing;
                        status = "";
                    }
                    try {
                        prepared.setVolume(vol, vol);
                        if (shouldStart) {
                            prepared.start();
                        }
                    } catch (Throwable ignored) {}
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
                            buffering = false;
                            playing = false;
                            status = "Playback error (" + what + ")";
                        }
                    }
                    releasePlayerOnThread();
                    return true;
                });
                synchronized (lock) {
                    if (gen != generation) {
                        try { mp.release(); } catch (Throwable ignored) {}
                        return;
                    }
                    player = mp;
                }
                mp.prepareAsync();
            } catch (Throwable e) {
                synchronized (lock) {
                    if (gen == generation) {
                        buffering = false;
                        playing = false;
                        status = "Audio unavailable: " + e.getClass().getSimpleName();
                    }
                }
                releasePlayerOnThread();
            }
        });
    }

    void togglePause() {
        final int gen;
        synchronized (lock) {
            if (urls.length == 0) return;
            gen = generation;
            playing = !playing;
            pausedByLifecycle = false;
        }
        final boolean targetPlay;
        synchronized (lock) { targetPlay = playing; }
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
                } else if (mp.isPlaying()) {
                    lastPositionMs = mp.getCurrentPosition();
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
            long pos = snapshotPositionLocked();
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
            durationMs = 0L;
            urls = new String[0];
            titles = new String[0];
            artists = new String[0];
            durationsMs = new long[0];
            currentIndex = 0;
            status = "";
        }
        handler.post(this::releasePlayerOnThread);
    }

    void onAppPause() {
        final int gen;
        synchronized (lock) {
            if (!playing) return;
            playing = false;
            pausedByLifecycle = true;
            gen = generation;
        }
        handler.post(() -> {
            MediaPlayer mp;
            synchronized (lock) {
                if (gen != generation || player == null || buffering) return;
                mp = player;
            }
            try {
                if (mp.isPlaying()) {
                    lastPositionMs = mp.getCurrentPosition();
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
            } catch (Throwable ignored) {}
        });
    }

    private long snapshotPositionLocked() {
        if (player != null && !buffering) {
            try {
                int pos = player.getCurrentPosition();
                if (pos >= 0) lastPositionMs = pos;
                int dur = player.getDuration();
                if (dur > 0) durationMs = dur;
            } catch (Throwable ignored) {}
        }
        return lastPositionMs;
    }

    String snapshot() {
        synchronized (lock) {
            long pos = snapshotPositionLocked();
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
        synchronized (lock) {
            mp = player;
            player = null;
        }
        if (mp != null) {
            try { mp.stop(); } catch (Throwable ignored) {}
            try { mp.reset(); } catch (Throwable ignored) {}
            try { mp.release(); } catch (Throwable ignored) {}
        }
    }
}
