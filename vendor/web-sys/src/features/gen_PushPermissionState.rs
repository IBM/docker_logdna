#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `PushPermissionState` enum."]
#[doc = ""]
#[doc = "*This API requires the following crate features to be activated: `PushPermissionState`*"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPermissionState {
    Granted = "granted",
    Denied = "denied",
    Prompt = "prompt",
}
