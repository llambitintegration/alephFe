use wasm_bindgen::prelude::*;

mod level;
mod mesh;
mod render;
mod sprites;
mod texture;

/// Entry point called from JavaScript after WASM loads.
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("Marathon Web initialized");
}

/// Start the game with pre-fetched scenario data.
/// Called from JS after fetching the binary files.
#[wasm_bindgen]
pub async fn start_game(
    map_data: &[u8],
    shapes_data: &[u8],
    physics_data: &[u8],
) -> Result<(), JsValue> {
    log::info!(
        "Starting game: map={}KB shapes={}KB physics={}KB",
        map_data.len() / 1024,
        shapes_data.len() / 1024,
        physics_data.len() / 1024,
    );

    render::run_web(map_data, shapes_data, physics_data)
        .await
        .map_err(|e| JsValue::from_str(&format!("Game error: {e}")))
}
