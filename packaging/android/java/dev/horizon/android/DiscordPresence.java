package dev.horizon.android;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.content.pm.ResolveInfo;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.util.Log;
import android.widget.Toast;

import com.discord.socialsdk.rpc.IDiscordRpcCallback;
import com.discord.socialsdk.rpc.IDiscordRpcConnection;
import com.discord.socialsdk.rpc.IDiscordRpcService;

import org.json.JSONObject;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

/**
 * Publishes the game's Rich Presence card on the player's Discord profile, from the phone.
 *
 * <p>The Discord Android app hosts a bound service ({@code
 * com.discord.socialsdk.rpc.IDiscordRpcService}, the transport its own Social SDK uses) that
 * turns JSON RPC frames into an activity card. This class binds that service — wherever it
 * answers, so a cloned or modified Discord works too — connects with the Volt+ application id
 * and hands over the card the game built. No account token is ever seen by this app.
 *
 * <p>Everything here runs on the main looper: the game thread only posts, so a slow or missing
 * Discord never costs the game a frame. A Discord that is not running, was killed, or refuses
 * the connection is retried every {@link #RETRY_MILLIS}, and the last card is sent again as soon
 * as the service says READY. When presence cannot work at all, the player is told once, with
 * the reason, instead of being left with a switch that silently does nothing.
 */
public final class DiscordPresence {
    private static final String TAG = "VoltDiscord";
    /** The bound service Discord's Social SDK publishes presence through. */
    private static final String ACTION = "com.discord.socialsdk.rpc.IDiscordRpcService";
    /** Tried by name when the package manager cannot be asked (some OEM builds answer neither). */
    private static final String[] CLIENTS = {
        "com.discord",
        "com.discord.beta",
        "com.discord.alpha",
        "com.discord.canary",
        "cocobo1.pupu.app",
        "com.aliucord",
        "app.revenge",
        "dev.bunny",
        "app.kettu"
    };
    private static final String RPC_VERSION = "1";
    /** Volt+ on Discord's developer portal (public: it names the card). */
    private static final long APPLICATION_ID = 1574331637087469588L;
    /**
     * The RPC pid: Discord only compares it with the nonce's answer, and the SDK's own frames
     * use 1 rather than the real process id.
     */
    private static final int HANDLER_PID = 1;
    private static final long RETRY_MILLIS = 15_000L;
    /** How long a fresh bind may take before the player is told it did not work. */
    private static final long EXPLAIN_AFTER_MILLIS = 6_000L;

    private static final Handler MAIN = new Handler(Looper.getMainLooper());

    private static Context app;
    private static IDiscordRpcService service;
    private static IDiscordRpcConnection connection;
    private static boolean bound;
    private static boolean ready;
    private static boolean wanted;
    /** The card to show: the last one the game built, resent on every READY. */
    private static String card;
    /**
     * How much of the card Discord takes: 0 everything, 1 without the party and the button,
     * 2 without pictures and links either. A client that answers an error steps down once.
     */
    private static int level;
    /** Why presence is not showing, for the one toast and the log. */
    private static String reason = "";
    private static boolean explained;
    /** Whether the player has already been told the card is up. Said once, like a failure. */
    private static boolean announced;

    private DiscordPresence() {}

    /** Shows {@code json} (the activity the game built). Safe from the game thread. */
    public static void publish(Context context, String json) {
        if (context == null || json == null || json.isEmpty()) {
            return;
        }
        remember(context);
        MAIN.post(
                () -> {
                    card = json;
                    wanted = true;
                    if (!bound) {
                        bind();
                    } else if (ready) {
                        send();
                    }
                });
    }

    /** Starts binding before any card exists, so DISCORD is ready when the first one arrives. */
    public static void start(Context context) {
        if (context == null) {
            return;
        }
        remember(context);
        MAIN.post(
                () -> {
                    wanted = true;
                    if (!bound) {
                        bind();
                    } else if (ready && card != null) {
                        send();
                    }
                });
    }

    /** Removes the card and lets Discord's service go. Safe from the game thread. */
    public static void clear() {
        MAIN.post(
                () -> {
                    MAIN.removeCallbacks(RETRY);
                    MAIN.removeCallbacks(EXPLAIN);
                    card = null;
                    wanted = false;
                    level = 0;
                    if (ready) {
                        flush(clearFrame());
                    }
                    detach();
                });
    }

    /** What the presence is doing, for the log and a future diagnostics row. */
    public static String status() {
        if (!wanted) {
            return "off";
        }
        if (ready) {
            return "showing";
        }
        if (bound) {
            return "connecting";
        }
        return reason.isEmpty() ? "waiting for Discord" : reason;
    }

    private static void remember(Context context) {
        Context application = context.getApplicationContext();
        app = application == null ? context : application;
    }

    private static void bind() {
        if (app == null || bound) {
            return;
        }
        List<Intent> candidates = candidateIntents();
        if (candidates.isEmpty()) {
            reason = "Discord does not offer its presence service on this device";
            explainOnce();
            retryLater();
            return;
        }
        Throwable lastError = null;
        for (Intent intent : candidates) {
            try {
                if (app.bindService(intent, CONNECTION, Context.BIND_AUTO_CREATE)) {
                    bound = true;
                    break;
                }
            } catch (Throwable error) {
                lastError = error;
                bound = false;
            }
        }
        if (!bound) {
            reason = lastError != null
                    ? "Discord refused the binding (" + lastError.getClass().getSimpleName() + ")"
                    : "Discord refused the presence service binding";
            explainOnce();
            retryLater();
            return;
        }
        MAIN.removeCallbacks(EXPLAIN);
        MAIN.postDelayed(EXPLAIN, EXPLAIN_AFTER_MILLIS);
    }

    /**
     * Every app on this device that offers the presence service: Discord itself, a clone, or a
     * client mod (they all keep the same action). The known packages are tried first.
     */
    private static List<Intent> candidateIntents() {
        List<String> seen = new ArrayList<>();
        List<Intent> intents = new ArrayList<>();
        for (String client : CLIENTS) {
            Intent intent = new Intent(ACTION).setPackage(client);
            if (resolves(intent) && !seen.contains(client)) {
                seen.add(client);
                intents.add(intent);
            }
        }
        for (String pkg : offeringPackages()) {
            if (seen.contains(pkg)) {
                continue;
            }
            Intent intent = new Intent(ACTION).setPackage(pkg);
            if (resolves(intent)) {
                seen.add(pkg);
                intents.add(intent);
            }
        }
        return intents;
    }

    private static boolean resolves(Intent intent) {
        try {
            return app.getPackageManager().resolveService(intent, 0) != null;
        } catch (Throwable ignored) {
            return false;
        }
    }

    private static List<String> offeringPackages() {
        List<String> packages = new ArrayList<>();
        try {
            List<ResolveInfo> found =
                    app.getPackageManager().queryIntentServices(new Intent(ACTION), 0);
            for (ResolveInfo info : found) {
                if (info.serviceInfo != null
                        && info.serviceInfo.packageName != null
                        && !packages.contains(info.serviceInfo.packageName)) {
                    packages.add(info.serviceInfo.packageName);
                }
            }
        } catch (Throwable ignored) {
            // No visibility of other packages: the two known ones were already tried.
        }
        return packages;
    }

    private static void connect() {
        if (service == null) {
            return;
        }
        IDiscordRpcConnection fresh;
        try {
            fresh = service.connect(APPLICATION_ID, RPC_VERSION, CALLBACK);
        } catch (Throwable error) {
            Log.i(TAG, "Discord refused the connection: " + error);
            reason = "Discord refused the connection (" + error.getClass().getSimpleName() + ")";
            fresh = null;
        }
        connection = fresh;
        ready = false;
        if (fresh == null) {
            detach();
            explainOnce();
            retryLater();
        }
    }

    /** Hands the current card to Discord; called once the service answered READY. */
    private static void send() {
        if (card == null || !ready) {
            return;
        }
        String activity = level == 0 ? card : reduced(card, level);
        if (activity == null) {
            return;
        }
        flush(cardFrame(activity));
        announceOnce();
    }

    /** The card without the fields a client refused: progressively strip optional fields. */
    private static String reduced(String activity, int depth) {
        try {
            JSONObject card = new JSONObject(activity);
            if (depth >= 1) {
                card.remove("buttons");
                card.remove("party");
                card.remove("details_url");
                card.remove("state_url");
                card.remove("status_display_type");
                JSONObject assets = card.optJSONObject("assets");
                if (assets != null) {
                    assets.remove("large_url");
                }
            }
            if (depth >= 2) {
                card.remove("assets");
            }
            if (depth >= 3) {
                card.remove("timestamps");
                card.remove("type");
                card.remove("name");
            }
            return card.toString();
        } catch (Throwable error) {
            return null;
        }
    }

    /** One frame over the binder; a Discord that went away is re-bound, never retried blindly. */
    private static void flush(String frame) {
        IDiscordRpcConnection target = connection;
        if (target == null || frame == null) {
            return;
        }
        try {
            target.sendFrame(frame);
        } catch (Throwable error) {
            Log.i(TAG, "Discord presence dropped: " + error);
            reason = "Discord dropped the connection (" + error.getClass().getSimpleName() + ")";
            detach();
            retryLater();
        }
    }

    /** {@code SET_ACTIVITY} with the game's card. */
    private static String cardFrame(String activity) {
        try {
            JSONObject args =
                    new JSONObject().put("pid", HANDLER_PID).put("activity", new JSONObject(activity));
            return new JSONObject()
                    .put("cmd", "SET_ACTIVITY")
                    .put("nonce", UUID.randomUUID().toString())
                    .put("args", args)
                    .toString();
        } catch (Throwable error) {
            // Malformed JSON is the game's bug: drop the card rather than send half of it.
            Log.i(TAG, "Discord presence card is not JSON: " + error);
            return null;
        }
    }

    /**
     * {@code SET_ACTIVITY} without an activity, which is how the Social SDK removes a card
     * ({@code args} holds only the pid).
     */
    private static String clearFrame() {
        try {
            return new JSONObject()
                    .put("cmd", "SET_ACTIVITY")
                    .put("nonce", UUID.randomUUID().toString())
                    .put("args", new JSONObject().put("pid", HANDLER_PID))
                    .toString();
        } catch (Throwable error) {
            return null;
        }
    }

    private static void detach() {
        ready = false;
        IDiscordRpcConnection closing = connection;
        connection = null;
        service = null;
        if (closing != null) {
            try {
                closing.disconnect();
            } catch (Throwable ignored) {
                // Already gone.
            }
        }
        if (bound) {
            try {
                if (app != null) {
                    app.unbindService(CONNECTION);
                }
            } catch (Throwable ignored) {
                // The binding was never established.
            }
            bound = false;
        }
    }

    /**
     * Confirms, once, that Discord took the card. Without it a working presence and a
     * silently ignored one look identical from this side of the screen.
     */
    private static void announceOnce() {
        if (announced || app == null) {
            return;
        }
        announced = true;
        String text = level == 0 ? "Discord presence is live" : "Discord presence is live (limited card)";
        Log.i(TAG, text);
        MAIN.post(
                () -> {
                    try {
                        Toast.makeText(app, text, Toast.LENGTH_LONG).show();
                    } catch (Throwable ignored) {
                        // No looper for a toast; the log carries it.
                    }
                });
    }

    /** One toast per session: a switch that does nothing is worse than a sentence. */
    private static void explainOnce() {
        if (explained || app == null) {
            return;
        }
        explained = true;
        String text = "Discord presence: " + status();
        Log.i(TAG, text);
        MAIN.post(
                () -> {
                    try {
                        Toast.makeText(app, text, Toast.LENGTH_LONG).show();
                    } catch (Throwable ignored) {
                        // A toast needs a running looper; the log has it either way.
                    }
                });
    }

    private static void retryLater() {
        if (!wanted) {
            return;
        }
        MAIN.removeCallbacks(RETRY);
        MAIN.postDelayed(RETRY, RETRY_MILLIS);
    }

    private static final Runnable RETRY = () -> {
        if (wanted && app != null && !bound) {
            bind();
        }
    };

    /** A bind that never produced a READY: say so, once, and keep retrying quietly. */
    private static final Runnable EXPLAIN =
            () -> {
                if (wanted && !ready) {
                    if (reason.isEmpty()) {
                        reason = "Discord did not answer its presence service";
                    }
                    explainOnce();
                }
            };

    private static final ServiceConnection CONNECTION =
            new ServiceConnection() {
                @Override
                public void onServiceConnected(ComponentName name, IBinder binder) {
                    service = IDiscordRpcService.Stub.asInterface(binder);
                    MAIN.post(DiscordPresence::connect);
                }

                @Override
                public void onServiceDisconnected(ComponentName name) {
                    MAIN.post(
                            () -> {
                                detach();
                                retryLater();
                            });
                }
            };

    private static final IDiscordRpcCallback.Stub CALLBACK =
            new IDiscordRpcCallback.Stub() {
                @Override
                public void onFrame(String frame) {
                    if (frame == null) {
                        return;
                    }
                    if (frame.contains("\"READY\"")) {
                        MAIN.post(
                                () -> {
                                    ready = true;
                                    level = 0;
                                    reason = "";
                                    MAIN.removeCallbacks(EXPLAIN);
                                    if (wanted) {
                                        send();
                                    }
                                });
                    } else if (frame.contains("\"ERROR\"")) {
                        // A card Discord did not like: step down instead of showing nothing.
                        Log.w(TAG, "Discord presence answered an error: " + frame);
                        MAIN.post(
                                () -> {
                                    if (card == null || level >= 3) {
                                        return;
                                    }
                                    level += 1;
                                    send();
                                    explainOnce();
                                });
                    }
                }

                @Override
                public void onClose(int code, String message) {
                    MAIN.post(
                            () -> {
                                Log.i(TAG, "Discord closed the presence (" + code + ": " + message + ")");
                                detach();
                                level = 0;
                                if (code == 4000) {
                                    reason = "Discord does not know this application id";
                                    explainOnce();
                                }
                                retryLater();
                            });
                }
            };
}
