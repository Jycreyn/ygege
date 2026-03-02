use crate::DOMAIN;
use crate::auth::login_with_flaresolverr;
use actix_web::{HttpRequest, HttpResponse, get};

#[get("/auth")]
pub async fn auth(req_data: HttpRequest) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let query = req_data.query_string();
    let qs = qstring::QString::from(query);
    let user: String = match qs.get("user") {
        Some(u) => u.to_string(),
        None => {
            return Ok(HttpResponse::BadRequest().body("Missing 'user' parameter"));
        }
    };
    let pass: String = match qs.get("pass") {
        Some(p) => p.to_string(),
        None => {
            return Ok(HttpResponse::BadRequest().body("Missing 'pass' parameter"));
        }
    };

    let res = login_with_flaresolverr(&user, &pass, false, None).await;
    match res {
        Ok(()) => {
            // Read session file if created
            let session_file = format!("sessions/{}.cookies", user);
            if let Ok(cookies) = std::fs::read_to_string(&session_file) {
                let mut response = HttpResponse::Ok();
                response.insert_header(("X-Session-Cookies", cookies.clone()));
                return Ok(response.body(cookies));
            }
            Ok(HttpResponse::Ok().body("Login successful"))
        }
        Err(e) => {
            error!("Login failed for user {}: {}", user, e);
            Ok(HttpResponse::Unauthorized().body(format!("Login failed: {}", e)))
        }
    }
}
