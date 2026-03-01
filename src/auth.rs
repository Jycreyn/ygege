use crate::domain::get_leaked_ip;
use crate::flaresolverr::FlareSolverrClient;
use crate::resolver::AsyncDNSResolverAdapter;
use crate::{DOMAIN, LOGIN_PAGE, LOGIN_PROCESS_PAGE};
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use wreq::header::HeaderMap;
use wreq::{Client, Url};
use wreq_util::{Emulation, EmulationOS, EmulationOption};

pub static KEY: OnceLock<String> = OnceLock::new();

pub async fn login(
    username: &str,
    password: &str,
    use_sessions: bool,
) -> Result<Client, Box<dyn std::error::Error>> {
    login_with_flaresolverr(username, password, use_sessions, None).await
}

pub async fn login_with_flaresolverr(
    username: &str,
    password: &str,
    use_sessions: bool,
    flaresolverr_url: Option<&str>,
) -> Result<Client, Box<dyn std::error::Error>> {
    debug!("Logging in with username: {}", username);

    let emu = EmulationOption::builder()
        .emulation(Emulation::Chrome132) // no H3 check on CF before 133
        .emulation_os(EmulationOS::Windows)
        .build();

    let domain_lock = DOMAIN.lock()?;
    let cloned_guard = domain_lock.clone();
    let domain = cloned_guard.as_str();
    drop(domain_lock);

    let leaked_ip = get_leaked_ip().await?;

    let client = Client::builder()
        .emulation(emu)
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .zstd(true)
        .cookie_store(true)
        .dns_resolver(Arc::new(AsyncDNSResolverAdapter::new()?))
        .cert_verification(false)
        .verify_hostname(false)
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(3))
        .resolve(
            &domain,
            SocketAddr::new(IpAddr::from_str(leaked_ip.as_str())?, 443),
        )
        .build()?;

    let mut headers = HeaderMap::new();
    add_bypass_headers(&mut headers);

    let start = std::time::Instant::now();

    if use_sessions {
        // check if the session file exists
        let session_file = format!("sessions/{}.cookies", username);
        if std::path::Path::new(&session_file.clone()).exists() {
            debug!("Session file found: {}", session_file);
            // load the session from the file
            let cookies = std::fs::read_to_string(&session_file)?;
            let cookies = cookies.split(";").collect::<Vec<&str>>();
            let cookies_len = cookies.len();
            for cookie in cookies {
                let cookie = cookie.trim();
                if cookie.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = cookie.split('=').collect();
                if parts.len() != 2 {
                    continue;
                }
                let name = parts[0].trim();
                let value = parts[1].trim();
                let cookie = wreq::cookie::CookieBuilder::new(name, value)
                    .domain(domain)
                    .path("/")
                    .http_only(true)
                    .secure(true)
                    .build();
                let url = Url::parse(format!("https://{domain}/").as_str())?;
                client.set_cookie(&url, cookie);
            }
            debug!("Restored {} cookies from session file", cookies_len);
        }

        // check if the session is still valid
        let session_check = client
            .get(format!("https://{domain}/"))
            .headers(headers.clone())
            .send()
            .await;
        match session_check {
            Ok(response) if response.status().is_success() => {
                let stop = std::time::Instant::now();
                debug!(
                    "Successfully resumed session in {:?}",
                    stop.duration_since(start)
                );
                return Ok(client);
            }
            Ok(response) => {
                debug!(
                    "Session is not valid, deleting session file (code {})",
                    response.status()
                );
                let _ = std::fs::remove_file(&session_file);
                debug!("Session file deleted");
            }
            Err(e) => {
                debug!(
                    "Session check failed ({}), deleting session file and proceeding to login",
                    e
                );
                let _ = std::fs::remove_file(&session_file);
            }
        }
    }

    client.clear_cookies();

    // inject account_created=true cookie (cookie magique)
    let cookie = wreq::cookie::CookieBuilder::new("account_created", "true")
        .domain(domain)
        .path("/")
        .http_only(true)
        .secure(true)
        .build();

    let url = Url::parse(format!("https://{domain}/").as_str())?;
    client.set_cookie(&url, cookie);

    // --- Étape 1 : Essayer de GET la page de login via wreq ---
    let login_page_url = format!("https://{domain}{LOGIN_PAGE}");
    let response = client
        .get(&login_page_url)
        .headers(headers.clone())
        .send()
        .await;

    // Déterminer si on a besoin de FlareSolverr
    let needs_flaresolverr = match &response {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!(
                    "Login page returned HTTP {} — possible Cloudflare block",
                    resp.status()
                );
                true
            } else {
                false
            }
        }
        Err(e) => {
            warn!("Login page request failed: {} — will try FlareSolverr", e);
            true
        }
    };

    // Vérifier le cookie ygg_ si la réponse est OK
    let has_ygg_cookie = if let Ok(ref resp) = response {
        if resp.status().is_success() {
            resp.cookies().any(|c| c.name() == "ygg_")
        } else {
            false
        }
    } else {
        false
    };

    // --- Étape 2 : FlareSolverr fallback si nécessaire ---
    // NOTE: Les cookies CF (cf_clearance) sont liés au fingerprint TLS du navigateur
    // qui les a obtenus. On ne peut PAS les transférer de FlareSolverr vers wreq.
    // Donc si wreq est bloqué, FlareSolverr doit faire le login COMPLET (GET + POST).
    if needs_flaresolverr || !has_ygg_cookie {
        if let Some(fs_url) = flaresolverr_url {
            warn!(
                "Cloudflare challenge detected (needs_flaresolverr={}, has_ygg_cookie={}), \
                 FlareSolverr will handle the full login at {}...",
                needs_flaresolverr, has_ygg_cookie, fs_url
            );

            let fs_client = FlareSolverrClient::new(fs_url)
                .map_err(|e| format!("Failed to create FlareSolverr client: {}", e))?;

            // Créer une session FlareSolverr persistante (les cookies survient entre requêtes)
            let session_id = fs_client
                .create_session()
                .await
                .map_err(|e| format!("Failed to create FlareSolverr session: {}", e))?;

            // Étape 2a : FlareSolverr GET la page de login (avec session)
            let get_solution = fs_client
                .solve_with_session(&login_page_url, 60000, Some(&session_id))
                .await
                .map_err(|e| format!("FlareSolverr GET login page failed: {}", e))?;

            info!(
                "FlareSolverr solved CF challenge! Got {} cookies",
                get_solution.cookies.len()
            );
            for cookie in &get_solution.cookies {
                debug!("  Cookie from GET: {}={}", cookie.name, cookie.value);
            }

            // Étape 2b : FlareSolverr POST les credentials (même session = cookies persistent)
            let post_url = format!("https://{domain}{LOGIN_PROCESS_PAGE}");
            let post_data = format!("id={}&pass={}", urlencoding::encode(username), urlencoding::encode(password));

            info!("FlareSolverr: POSTing credentials to {}...", post_url);
            let post_solution = fs_client
                .solve_post(&post_url, &post_data, None, 60000, Some(&session_id))
                .await
                .map_err(|e| format!("FlareSolverr POST login failed: {}", e))?;

            info!(
                "FlareSolverr login POST completed! Status: {}, got {} cookies",
                post_solution.status,
                post_solution.cookies.len()
            );

            // Store the User-Agent used by FlareSolverr to solve the CF challenge
            // This is required because wreq needs to send the EXACT same User-Agent
            // when using the cf_clearance cookie to download binary torrents.
            FlareSolverrClient::set_user_agent(post_solution.user_agent.clone()).await;

            // Injecter TOUS les cookies du POST dans le client wreq
            // Ces cookies incluent les cookies de session YGG (pas juste cf_clearance)
            let base_url = Url::parse(&format!("https://{domain}/"))?;
            for cookie in &post_solution.cookies {
                debug!(
                    "Injecting session cookie: {}={} (domain: {})",
                    cookie.name, cookie.value, cookie.domain
                );
                let c = wreq::cookie::CookieBuilder::new(
                    cookie.name.as_str(),
                    cookie.value.as_str(),
                )
                .domain(domain)
                .path(&cookie.path)
                .http_only(true)
                .secure(true)
                .build();
                client.set_cookie(&base_url, c);
            }

            let stop = std::time::Instant::now();
            info!(
                "Logged in successfully via FlareSolverr in {:?}",
                stop.duration_since(start)
            );

            // Sauvegarder la session
            if use_sessions {
                save_session(username, &client).await?;
            }

            return Ok(client);
        } else {
            // Pas de FlareSolverr configuré
            if needs_flaresolverr {
                return Err(
                    "Cloudflare blocked the login page and FLARESOLVERR_URL is not set. \
                     Set FLARESOLVERR_URL to enable automatic bypass."
                        .into(),
                );
            } else {
                return Err("No ygg_ cookie found and FLARESOLVERR_URL is not set".into());
            }
        }
    } else {
        debug!("Login page fetched successfully with ygg_ cookie via wreq (no FlareSolverr needed)");
    }

    // --- Étape 3 (wreq only) : POST credentials ---
    let payload = [("id", username), ("pass", password)];

    let response = client
        .post(format!("https://{domain}{LOGIN_PROCESS_PAGE}"))
        .headers(headers.clone())
        .form(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        if response.status() == 401 {
            error!("Invalid username or password");
            return Err("Invalid username or password".into());
        }
        return Err(format!("Failed to login: {}", response.status()).into());
    }

    let _headers = response.headers();

    // get site root page for final cookies
    let response = client
        .get(format!("https://{domain}/"))
        .headers(headers.clone())
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch site root page: {}", response.status()).into());
    }

    let stop = std::time::Instant::now();
    debug!("Logged in successfully via wreq in {:?}", stop.duration_since(start));

    let _headers = response.cookies();

    if use_sessions {
        save_session(username, &client).await?;
    }

    Ok(client)
}

async fn save_session(username: &str, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // save the session in a file
    let mut file = File::create(format!("sessions/{}.cookies", username))?;
    let cookies_header = client
        .get_cookies(&Url::parse(
            format!("https://{}/", DOMAIN.lock()?.as_str()).as_str(),
        )?)
        .unwrap();
    let cookies_header_value = cookies_header.to_str()?;
    debug!("Cookies: {}", cookies_header_value);
    file.write_all(cookies_header_value.as_bytes())?;
    file.flush()?;

    Ok(())
}

pub fn add_bypass_headers(headers: &mut HeaderMap) {
    let own_ip_lock = crate::domain::OWN_IP.get();
    if let Some(own_ip) = own_ip_lock {
        headers.insert("CF-Connecting-IP", own_ip.parse().unwrap());
        headers.insert("X-Forwarded-For", own_ip.parse().unwrap());
    }
}
