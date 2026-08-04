use hyper::Request;
use tower_http::request_id::{MakeRequestId, RequestId};

#[derive(Clone, Copy)]
pub struct MakeUuidV7;

impl MakeRequestId for MakeUuidV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let uuid = uuid::Uuid::now_v7();
        axum::http::HeaderValue::from_str(&uuid.to_string())
            .ok()
            .map(RequestId::new)
    }
}
