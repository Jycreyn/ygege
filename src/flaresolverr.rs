use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// Global FlareSolverr client, initialized if FLARESOLVERR_URL is set
pub static FLARESOLVERR: OnceLock<FlareSolverrClient> = OnceLock::new();

/// Global FlareSolverr session ID (persists cookies across requests)
static FS_SESSION_ID: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// Global FlareSolverr User-Agent (must match when doing wreq calls with CF cookies)
static FS_USER_AGENT: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn get_session_lock() -> &'static RwLock<Option<String>> {
    FS_SESSION_ID.get_or_init(|| RwLock::new(None))
}

fn get_user_agent_lock() -> &'static RwLock<Option<String>> {
    FS_USER_AGENT.get_or_init(|| RwLock::new(None))
}

/// Client FlareSolverr pour bypasser Cloudflare en fallback
pub struct FlareSolverrClient {
    url: String,
    client: wreq::Client,
}

// --- Request/Response structs ---

#[derive(Debug, Serialize)]
struct FlareSolverrGetRequest<'a> {
    cmd: &'a str,
    url: &'a str,
    #[serde(rename = "maxTimeout")]
    max_timeout: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
}


#[derive(Debug, Serialize)]
struct FlareSolverrPostRequest<'a> {
    cmd: &'a str,
    url: &'a str,
    #[serde(rename = "maxTimeout")]
    max_timeout: u64,
    #[serde(rename = "postData")]
    post_data: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cookies: Option<Vec<FlareSolverrCookieInput<'a>>>,
}

#[derive(Debug, Serialize)]
struct FlareSolverrSessionRequest<'a> {
    cmd: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct FlareSolverrCookieInput<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub domain: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct FlareSolverrResponse {
    pub status: String,
    pub solution: Option<FlareSolverrSolution>,
    pub message: Option<String>,
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FlareSolverrSolution {
    pub url: String,
    pub status: u16,
    pub cookies: Vec<FlareSolverrCookie>,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    pub response: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FlareSolverrCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

impl FlareSolverrClient {
    pub fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = wreq::Client::builder().build()?;
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            client,
        })
    }

    /// Initialize the global FlareSolverr client
    pub fn init_global(url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::new(url)?;
        FLARESOLVERR
            .set(client)
            .map_err(|_| "FlareSolverr already initialized")?;
        Ok(())
    }

    pub fn is_available() -> bool {
        FLARESOLVERR.get().is_some()
    }

    /// Create a persistent FlareSolverr session (reuses browser + cookies)
    pub async fn create_session(&self) -> Result<String, Box<dyn std::error::Error>> {
        let endpoint = format!("{}/v1", self.url);
        let body = FlareSolverrSessionRequest {
            cmd: "sessions.create",
            session: None,
        };

        debug!("FlareSolverr: creating persistent session...");
        let response = self.client.post(&endpoint).json(&body).send().await?;
        let fs_resp: FlareSolverrResponse = response.json().await?;

        let session_id = fs_resp
            .session
            .ok_or("FlareSolverr did not return a session ID")?;
        info!("FlareSolverr session created: {}", session_id);

        // Store globally
        let mut lock = get_session_lock().write().await;
        *lock = Some(session_id.clone());

        Ok(session_id)
    }

    /// Get the current global session ID (if any)
    pub async fn get_session_id() -> Option<String> {
        let lock = get_session_lock().read().await;
        lock.clone()
    }

    pub async fn set_user_agent(ua: String) {
        let mut lock = get_user_agent_lock().write().await;
        *lock = Some(ua);
    }

    pub async fn get_user_agent() -> Option<String> {
        let lock = get_user_agent_lock().read().await;
        lock.clone()
    }

    /// Fetch a page via FlareSolverr using the persistent session
    pub async fn fetch_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
        Self::fetch_page_with_solution(url).await.map(|s| s.response)
    }

    /// Fetch a page via FlareSolverr, returning the full solution (with cookies + user-agent)
    pub async fn fetch_page_with_solution(url: &str) -> Result<FlareSolverrSolution, Box<dyn std::error::Error>> {
        let fs = FLARESOLVERR
            .get()
            .ok_or("FlareSolverr not configured (set FLARESOLVERR_URL)")?;

        // Get or create session
        let session_id = match Self::get_session_id().await {
            Some(id) => id,
            None => {
                info!("No FlareSolverr session exists, creating one...");
                fs.create_session().await?
            }
        };

        fs.solve_with_session(url, 60000, Some(&session_id)).await
    }

    /// GET with optional session
    pub async fn solve_with_session(
        &self,
        target_url: &str,
        max_timeout: u64,
        session: Option<&str>,
    ) -> Result<FlareSolverrSolution, Box<dyn std::error::Error>> {
        let endpoint = format!("{}/v1", self.url);
        let request_body = FlareSolverrGetRequest {
            cmd: "request.get",
            url: target_url,
            max_timeout,
            session,
        };

        debug!("FlareSolverr: request.get {} (session: {:?})", target_url, session.map(|s| &s[..s.len().min(12)]));

        let response = self.client.post(&endpoint).json(&request_body).send().await?;

        if !response.status().is_success() {
            return Err(format!("FlareSolverr returned HTTP {}", response.status()).into());
        }

        let fs_response: FlareSolverrResponse = response.json().await?;
        Self::extract_solution(fs_response)
    }

    /// GET without session (for initial login)
    pub async fn solve(
        &self,
        target_url: &str,
        max_timeout: u64,
    ) -> Result<FlareSolverrSolution, Box<dyn std::error::Error>> {
        self.solve_with_session(target_url, max_timeout, None).await
    }


    /// POST with optional session and cookies
    pub async fn solve_post(
        &self,
        target_url: &str,
        post_data: &str,
        cookies: Option<Vec<FlareSolverrCookieInput<'_>>>,
        max_timeout: u64,
        session: Option<&str>,
    ) -> Result<FlareSolverrSolution, Box<dyn std::error::Error>> {
        let endpoint = format!("{}/v1", self.url);
        let request_body = FlareSolverrPostRequest {
            cmd: "request.post",
            url: target_url,
            max_timeout,
            post_data,
            session,
            cookies,
        };

        info!("FlareSolverr: request.post {} (session: {:?})", target_url, session.map(|s| &s[..s.len().min(12)]));

        let response = self.client.post(&endpoint).json(&request_body).send().await?;

        if !response.status().is_success() {
            return Err(format!("FlareSolverr POST returned HTTP {}", response.status()).into());
        }

        let fs_response: FlareSolverrResponse = response.json().await?;
        Self::extract_solution(fs_response)
    }

    fn extract_solution(
        fs_response: FlareSolverrResponse,
    ) -> Result<FlareSolverrSolution, Box<dyn std::error::Error>> {
        if fs_response.status != "ok" {
            let msg = fs_response
                .message
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(format!("FlareSolverr error: {}", msg).into());
        }

        fs_response
            .solution
            .ok_or_else(|| "FlareSolverr returned no solution".into())
    }
}


/// Centralized function to fetch a YGG YGG page.
/// Tries wreq first, falls back to FlareSolverr (with persistent session) if CF blocks.
pub async fn fetch_ygg_page(
    client: &wreq::Client,
    url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Try wreq first
    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                return response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e).into());
            }

            // CF block — fallback to FlareSolverr with session
            if status.as_u16() == 307 || status.as_u16() == 302
                || status.as_u16() == 403 || status.as_u16() == 503
            {
                warn!(
                    "wreq blocked by CF (HTTP {}) for {} — falling back to FlareSolverr",
                    status, url
                );
                if FlareSolverrClient::is_available() {
                    return FlareSolverrClient::fetch_page(url).await;
                } else {
                    return Err(format!(
                        "CF blocked (HTTP {}) and FlareSolverr not configured", status
                    ).into());
                }
            }

            Err(format!("HTTP error {} for {}", status, url).into())
        }
        Err(e) => {
            warn!("wreq request failed for {}: {} — trying FlareSolverr", url, e);
            if FlareSolverrClient::is_available() {
                FlareSolverrClient::fetch_page(url).await
            } else {
                Err(format!("Request failed and FlareSolverr not configured: {}", e).into())
            }
        }
    }
}

/// Centralized function to fetch a YGG POST page (e.g., token).
/// Tries wreq first, falls back to FlareSolverr (with persistent session) if CF blocks.
pub async fn fetch_ygg_post(
    client: &wreq::Client,
    url: &str,
    post_data: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut headers = wreq::header::HeaderMap::new();
    headers.insert(
        "Content-Type",
        "application/x-www-form-urlencoded; charset=UTF-8".parse().unwrap(),
    );

    // Try wreq first
    match client.post(url).headers(headers).body(post_data.to_string()).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                return response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e).into());
            }

            // CF block — fallback to FlareSolverr with session
            if status.as_u16() == 307 || status.as_u16() == 302
                || status.as_u16() == 403 || status.as_u16() == 503
            {
                warn!(
                    "wreq blocked by CF (HTTP {}) for POST {} — falling back to FlareSolverr",
                    status, url
                );
                if FlareSolverrClient::is_available() {
                    let fs = FLARESOLVERR.get().unwrap();
                    let session_id = match FlareSolverrClient::get_session_id().await {
                        Some(id) => id,
                        None => fs.create_session().await?
                    };
                    let solution = fs.solve_post(url, post_data, None, 60000, Some(&session_id)).await?;
                    return Ok(solution.response);
                } else {
                    return Err(format!(
                        "CF blocked (HTTP {}) and FlareSolverr not configured", status
                    ).into());
                }
            }

            Err(format!("HTTP error {} for {}", status, url).into())
        }
        Err(e) => {
            warn!("wreq POST request failed for {}: {} — trying FlareSolverr", url, e);
            if FlareSolverrClient::is_available() {
                let fs = FLARESOLVERR.get().unwrap();
                let session_id = match FlareSolverrClient::get_session_id().await {
                    Some(id) => id,
                    None => fs.create_session().await?
                };
                let solution = fs.solve_post(url, post_data, None, 60000, Some(&session_id)).await?;
                Ok(solution.response)
            } else {
                Err(format!("Request failed and FlareSolverr not configured: {}", e).into())
            }
        }
    }
}
