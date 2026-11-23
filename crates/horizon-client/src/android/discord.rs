//! Discord Rich Presence on Android: JNI calls into `dev.horizon.android.DiscordPresence`.
//!
//! A phone has no Discord IPC socket to talk to, so the desktop transport in
//! `horizon_client::discord_presence` cannot work here. The Discord app does publish presence
//! through a bound service of its own, though, and the Java side binds it: this module only
//! hands over the card the game built ("super detailed" lives in the shared module, so the
//! phone and the desktop say the same thing).
use jni::objects::{JObject, JValue};

const CLASS_NAME: &str = "dev.horizon.android.DiscordPresence";

/// Installs the phone's transport, once, before the game starts publishing. Automated runs
/// stay silent, exactly like the desktop thread; the switch's own value is the game's business.
pub fn install(automated: bool) {
    if automated {
        return;
    }
    horizon_client::discord_presence::use_platform(horizon_client::discord_presence::Sink {
        publish,
        clear,
    });
    // Bind now rather than on the first card: Discord's READY takes a moment, and the title
    // screen's card should not be the thing that pays for it.
    if crate::platform::with_context(|env, activity| {
        let class = crate::platform::app_class(env, activity, CLASS_NAME)?;
        env.call_static_method(
            class,
            "start",
            "(Landroid/content/Context;)V",
            &[JValue::Object(activity)],
        )?;
        Ok(())
    })
    .is_none()
    {
        log::warn!("VoltDiscord install skipped or failed");
    }
}

/// One card, as the game built it. Discord's card needs the activity, the app id is the Java
/// side's business, and the binder call is left to the main looper there.
fn publish(card: &str) {
    if crate::platform::with_context(|env, activity| {
        let class = crate::platform::app_class(env, activity, CLASS_NAME)?;
        let text = JObject::from(env.new_string(card)?);
        env.call_static_method(
            class,
            "publish",
            "(Landroid/content/Context;Ljava/lang/String;)V",
            &[JValue::Object(activity), JValue::Object(&text)],
        )?;
        Ok(())
    })
    .is_none()
    {
        log::warn!("VoltDiscord publish skipped or failed");
    }
}

/// Takes the card off the profile (the switch turned off, or the game is quitting).
fn clear() {
    if crate::platform::with_context(|env, activity| {
        let class = crate::platform::app_class(env, activity, CLASS_NAME)?;
        env.call_static_method(class, "clear", "()V", &[])?;
        Ok(())
    })
    .is_none()
    {
        log::warn!("VoltDiscord clear skipped or failed");
    }
}
