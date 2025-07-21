use serde::{Deserialize, Serialize};
use web_sys::window;

pub async fn delay_with_message(duration_ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = window() {
            let _ =
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration_ms);
        }
    });

    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkerMessage {
    Generated {
        public_key: String,
        private_key: String,
    },
}
