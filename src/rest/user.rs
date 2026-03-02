use crate::config::Config;
use crate::rest::client_extractor::MaybeCustomClient;
use actix_web::{HttpResponse, get, web};

#[get("/user")]
pub async fn get_user_info(
    data: MaybeCustomClient,
    config: web::Data<Config>,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let user = crate::user::get_account().await;
    // check if error is session expired
    if let Err(e) = &user {
        if e.to_string().contains("Session expired") && !data.is_custom {
            info!("Trying to renew session...");
            let _ = crate::auth::login_with_flaresolverr(
                config.username.as_str(),
                config.password.as_str(),
                true,
                config.flaresolverr_url.as_deref(),
            )
            .await?;

            info!("Session renewed via FlareSolverr, retrying to get user info...");
            let user = crate::user::get_account().await?;
            let json = serde_json::to_value(&user)?;
            let mut response = HttpResponse::Ok();
            if let Some(cookies) = data.cookies_header {
                response.insert_header(("X-Session-Cookies", cookies));
            }
            return Ok(response.json(json));
        }
    }

    let user = user?;
    let json = serde_json::to_value(&user)?;
    let mut response = HttpResponse::Ok();
    if let Some(cookies) = data.cookies_header {
        response.insert_header(("X-Session-Cookies", cookies));
    }
    Ok(response.json(json))
}
