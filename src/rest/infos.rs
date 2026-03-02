use crate::config::Config;
use crate::rest::client_extractor::MaybeCustomClient;
use crate::search::{Order, Sort, search};
use crate::utils::get_remaining_downloads;
use crate::DOMAIN;
use actix_web::{HttpResponse, get, web};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use trust_dns_resolver::TokioAsyncResolver;

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[get("/status")]
pub async fn status_check(data: MaybeCustomClient, config: web::Data<Config>) -> HttpResponse {
    let domain_lock = DOMAIN.lock().unwrap();
    let cloned_guard = domain_lock.clone();
    let domain = cloned_guard.as_str();
    drop(domain_lock);

    let search = search(
        "Vaiana",
        None,
        None,
        None,
        Some(Sort::Seed),
        Some(Order::Ascending),
        None,
        false,
    )
    .await;

    let auth: &str;
    let search_status: &str;
    let parsing: &str;
    match search {
        Ok(torrents) => {
            auth = "authenticated";
            search_status = "ok";
            if torrents.is_empty() {
                parsing = "failed";
            } else {
                parsing = "ok";
            }
        }
        Err(e) => {
            if e.to_string().contains("Session expired") {
                auth = "not_authenticated";
                search_status = "ok";
                parsing = "n/a";
            } else {
                error!("Status check auth error: {}", e);
                auth = "auth_error";
                search_status = "failed";
                parsing = "n/a";
            }
        }
    }

    let user = crate::user::get_account().await;
    let user_status = match user.is_ok() {
        true => "ok",
        false => "failed",
    };

    let mut domain_ping = "unreachable";
    let dns_lookup = match TokioAsyncResolver::tokio_from_system_conf()
        .and_then(|r| futures::executor::block_on(r.lookup_ip(domain.clone())))
    {
        Ok(lookup) => {
            if let Some(ip_addr) = lookup.iter().next() {
                info!("Resolved IP: {}", ip_addr);
                let socket_addr = SocketAddr::new(ip_addr, 443);
                domain_ping = match timeout(Duration::from_secs(5), TcpStream::connect(socket_addr)).await {
                    Ok(Ok(_)) => {
                        info!("TCP connection to {} successful", socket_addr);
                        "reachable"
                    }
                    Ok(Err(e)) => {
                        error!("TCP connection failed: {}", e);
                        "unreachable"
                    }
                    Err(_) => {
                        error!("TCP connection timeout");
                        "timeout"
                    }
                };
            }
            "resolves"
        }
        Err(_) => "does_not_resolve",
    };

    let tmdb = match config.tmdb_token.is_some() {
        true => "enabled",
        false => "disabled",
    };

    let remain = match get_remaining_downloads().await {
        Ok(n) => n as i32,
        Err(e) => {
            error!("Failed to get remaining downloads: {}", e);
            -1
        }
    };

    let status = serde_json::json!({
        "domain": domain,
        "auth": auth,
        "search": search_status,
        "user_info": user_status,
        "domain_reachability": domain_ping,
        "domain_dns": dns_lookup,
        "parsing": parsing,
        "tmdb_integration": tmdb,
        "remaining_downloads": remain,
    });

    let mut response = HttpResponse::Ok();
    if let Some(cookies) = data.cookies_header {
        response.insert_header(("X-Session-Cookies", cookies));
    }
    response.json(status)
}
