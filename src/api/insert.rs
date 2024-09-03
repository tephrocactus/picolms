use poem::handler;
use poem::http::StatusCode;

#[handler]
pub fn handler() -> StatusCode {
    StatusCode::NO_CONTENT
}
