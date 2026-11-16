#[cfg(target_arch = "wasm32")]
pub fn is_touch_device() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(pointer: coarse)").ok().flatten())
        .is_some_and(|m| m.matches())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn is_touch_device() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
pub fn prompt(label: &str, current: &str) -> Option<String> {
    web_sys::window()?
        .prompt_with_message_and_default(label, current)
        .ok()
        .flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn prompt(label: &str, current: &str) -> Option<String> {
    None
}
