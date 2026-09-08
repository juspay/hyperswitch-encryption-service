use std::sync::Arc;

use axum::{Router, routing::post};

use crate::{app::AppState, core};
pub struct DataKey;

impl DataKey {
    pub fn server(state: Arc<AppState>) -> Router<Arc<AppState>> {
        let router = Router::new()
            .route("/create", post(core::create_data_key))
            .route("/rotate", post(core::rotate_data_key))
            .route("/transfer", post(core::transfer_data_key))
            .route("/list", post(core::datakey::list_data_keys_handler));

        #[cfg(feature = "aws")]
        let router = router.route(
            "/reencrypt",
            post(core::datakey::reencrypt_data_keys_handler),
        );

        router.with_state(state)
    }
}
