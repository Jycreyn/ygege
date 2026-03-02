use crate::domain::get_leaked_ip;
use crate::flaresolverr::{FlareSolverrClient, FLARESOLVERR};
use crate::{DOMAIN, LOGIN_PAGE, LOGIN_PROCESS_PAGE};
use std::fs::File;
use std::io::Write;
use std::sync::OnceLock;

pub static KEY: OnceLock<String> = OnceLock::new();

pub async fn login(
    username: &str,
    password: &str,
    use_sessions: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // For backwards compatibility the caller may pass None for flaresolverr_url,
    // but FlareSolverr is required for all HTTP interactions after removing wreq.
    Err("Use login_with_flaresolverr and provide FLARESOLVERR_URL in config".into())
}

pub async fn login_with_flaresolverr(
    username: &str,
    password: &str,
    use_sessions: bool,
    flaresolverr_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Logging in with username: {} via FlareSolverr", username);

    let domain_lock = DOMAIN.lock()?;
    let domain = domain_lock.clone();
    drop(domain_lock);

    // Ensure we can determine leaked IP (used previously for DNS resolve logic)
    let _ = get_leaked_ip().await?;

    // Determine FlareSolverr client: either provided URL or existing global
    if flaresolverr_url.is_some() || FLARESOLVERR.get().is_some() {
        let fs_client_ref: &FlareSolverrClient = if let Some(fs_url) = flaresolverr_url {
            // create a temporary client when URL provided
            Box::leak(Box::new(
                FlareSolverrClient::new(fs_url)
                    .map_err(|e| format!("Failed to create FlareSolverr client: {}", e))?,
            ))
        } else {
            FLARESOLVERR.get().unwrap()
        };

        // Create persistent session
        let session_id = fs_client_ref
            .create_session()
            .await
            .map_err(|e| format!("Failed to create FlareSolverr session: {}", e))?;

        // GET the login page with session
        let login_page_url = format!("https://{}{}", domain, LOGIN_PAGE);
        let _get_solution = fs_client_ref
            .solve_with_session(&login_page_url, 60000, Some(&session_id))
            .await
            .map_err(|e| format!("FlareSolverr GET login page failed: {}", e))?;

        // POST credentials with same session
        let post_url = format!("https://{}{}", domain, LOGIN_PROCESS_PAGE);
        let post_data = format!("id={}&pass={}", urlencoding::encode(username), urlencoding::encode(password));

        let post_solution = fs_client_ref
            .solve_post(&post_url, &post_data, None, 60000, Some(&session_id))
            .await
            .map_err(|e| format!("FlareSolverr POST login failed: {}", e))?;

        // Store UA used by FlareSolverr
        FlareSolverrClient::set_user_agent(post_solution.user_agent.clone()).await;

        // Optionally save session cookies to file so user can re-use them externally
        if use_sessions {
            let cookie_string = post_solution
                .cookies
                .iter()
                .map(|c| format!("{}={}", c.name, c.value))
                .collect::<Vec<_>>()
                .join("; ");
            let mut file = File::create(format!("sessions/{}.cookies", username))?;
            file.write_all(cookie_string.as_bytes())?;
            file.flush()?;
        }

        info!("Logged in successfully via FlareSolverr");
        Ok(())
    } else {
        Err("FLARESOLVERR_URL is not set; FlareSolverr is required after removing wreq".into())
    }
}
