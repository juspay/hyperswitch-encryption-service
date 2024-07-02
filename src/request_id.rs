use hyper::Request;
use tower_http::request_id::{MakeRequestId, RequestId};

#[derive(Clone)]
pub struct MakeUlid;

impl MakeRequestId for MakeUlid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let ulid = ulid::Ulid::new().to_string();

        Some(RequestId::new(ulid.parse().ok()?))
    }
}
