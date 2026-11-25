//! JNI input edge handoff for Android control overlays.
//! All game mutation happens later on the winit thread.
use jni::{
    JNIEnv,
    objects::{JObject, JString},
    sys::{JNI_FALSE, JNI_TRUE, jboolean},
};
use horizon_client::java_controls;

/// The string protocol keeps launcher actions distinct from keyboard keys.
/// Return false for actions that must be handled by the Android overlay itself.
pub extern "system" fn control_input(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    kind: JString<'_>,
    key: JString<'_>,
    down: jboolean,
) -> jboolean {
    if kind.is_null() || key.is_null() { return JNI_FALSE; }
    let result = (|| -> jni::errors::Result<_> {
        let kind: String = env.get_string(&kind)?.into();
        let key: String = env.get_string(&key)?.into();
        let pressed = down != JNI_FALSE;
        if kind == "pojav" {
            let supported = key.parse().ok()
                .is_some_and(|code| java_controls::submit_pojav_event(code, pressed));
            return Ok(if supported { JNI_TRUE } else { JNI_FALSE });
        }
        let handled = java_controls::zalith_event(&kind, &key)
            .is_some_and(|event| java_controls::submit_control_event(event, pressed));
        Ok(if handled { JNI_TRUE } else { JNI_FALSE })
    })();
    match result {
        Ok(value) => value,
        Err(_) => { let _ = env.exception_clear(); JNI_FALSE }
    }
}

pub extern "system" fn control_reset(_env: JNIEnv<'_>, _activity: JObject<'_>) {
    java_controls::release_tokens();
}
