use std::sync::OnceLock;
use crate::flaresolverr::FlareSolverrClient;

const CURRENT_REDIRECT_DOMAINS: [&str; 4] =
    ["yggtorrent.ch", "ygg.to", "yggtorrent.to", "yggtorrent.is"];

pub static OWN_IP: OnceLock<String> = OnceLock::new();

pub async fn get_ygg_domain() -> Result<String, Box<dyn std::error::Error>> {
    debug!("Getting YGG current domain by trying all base domains in parallel via FlareSolverr");

    let start = std::time::Instant::now();

    let mut tasks = Vec::new();
    for &base_domain in &CURRENT_REDIRECT_DOMAINS {
        let domain_clone = base_domain.to_string();
        let task = tokio::spawn(async move {
            // Use FlareSolverr to resolve redirects/CF
            let url = format!("https://{}", domain_clone);
            let solution_res = FlareSolverrClient::fetch_page_with_solution(&url).await;
            let solution = match solution_res {
                Ok(s) => s,
                Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            };
            let resolved = solution
                .url
                .split('/')
                .nth(2)
                .ok_or("No domain found")?
                .to_string();
            Ok::<String, Box<dyn std::error::Error + Send + Sync>>(resolved)
        });
        tasks.push(task);
    }

    let mut last_error = None;
    while !tasks.is_empty() {
        let (result, _idx, remaining) = futures::future::select_all(tasks).await;
        tasks = remaining;

        match result {
            Ok(Ok(domain)) => {
                let stop = std::time::Instant::now();
                debug!(
                    "Found current YGG domain: {} in {:?}",
                    domain,
                    stop.duration_since(start)
                );
                return Ok(domain);
            }
            Ok(Err(e)) => {
                debug!("Failed to get domain from one source: {}", e);
                last_error = Some(e);
            }
            Err(e) => {
                debug!("Task panicked: {}", e);
                last_error = Some(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "All domain checks failed".into()))
}

pub async fn get_own_ip() -> Result<String, Box<dyn std::error::Error>> {
    let body = FlareSolverrClient::fetch_page("https://api64.ipify.org?format=text").await?;
    Ok(body)
}

pub async fn get_leaked_ip() -> Result<String, Box<dyn std::error::Error>> {
    let body = FlareSolverrClient::fetch_page("https://pastebin.com/raw/jFZt5UHb").await?;
    Ok(body.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_ygg_domain() {
        let result = get_ygg_domain().await;
        assert!(result.is_ok());
        let domain = result.unwrap();
        assert!(!domain.is_empty());
        println!("YGG Domain: {}", domain);
    }
}
