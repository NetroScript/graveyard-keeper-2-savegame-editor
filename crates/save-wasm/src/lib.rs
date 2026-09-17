use gk2_save_core::Service;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SaveSession {
    inner: Service,
}

#[wasm_bindgen]
impl SaveSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Service::default(),
        }
    }
    pub fn open(&mut self, bytes: &[u8]) -> Result<String, JsValue> {
        let summary = self.inner.open(bytes).map_err(js_error)?;
        serde_json::to_string(&summary).map_err(js_error)
    }
    pub fn request(&mut self, request: &str) -> Result<String, JsValue> {
        let request = serde_json::from_str(request).map_err(js_error)?;
        let response = self.inner.request(request).map_err(js_error)?;
        serde_json::to_string(&response).map_err(js_error)
    }
    pub fn export(&self) -> Result<Vec<u8>, JsValue> {
        self.inner.export().map_err(js_error)
    }
}
impl Default for SaveSession {
    fn default() -> Self {
        Self::new()
    }
}
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
