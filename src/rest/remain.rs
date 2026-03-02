use actix_web::{HttpResponse, get};

use crate::rest::client_extractor::MaybeCustomClient;
use crate::utils::get_remaining_downloads;

#[get("/remain")]
pub async fn remaining_downloads_status(data: MaybeCustomClient) -> HttpResponse {
    let remain = match get_remaining_downloads().await {
        Ok(n) => n as i32,
        Err(e) => {
            if e.to_string().contains("Session expired") {
                return HttpResponse::Unauthorized().body("Session expired");
            }
            error!("Failed to get remaining downloads: {}", e);
            -1
        }
    };

    let mut response = HttpResponse::Ok();
    if let Some(cookies) = data.cookies_header {
        response.insert_header(("X-Session-Cookies", cookies));
    }
    response.body(remain.to_string())
}
