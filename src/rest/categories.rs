use crate::categories::CATEGORIES_CACHE;
use crate::rest::client_extractor::MaybeCustomClient;
use actix_web::{HttpResponse, get};

#[get("/categories")]
pub async fn categories(data: MaybeCustomClient) -> HttpResponse {
    // Try to get from cache first
    if let Some(cached_categories) = CATEGORIES_CACHE.get() {
        let mut response = HttpResponse::Ok();
        if let Some(cookies) = data.cookies_header {
            response.insert_header(("X-Session-Cookies", cookies));
        }
        return response.json(cached_categories);
    }

    // If cache is empty (shouldn't happen after startup), scrape now
    warn!("Categories cache was empty, scraping now...");
    match crate::categories::scrape_categories().await {
        Ok(categories) => {
            let _ = CATEGORIES_CACHE.set(categories.clone());
            let mut response = HttpResponse::Ok();
            if let Some(cookies) = data.cookies_header {
                response.insert_header(("X-Session-Cookies", cookies));
            }
            response.json(categories)
        }
        Err(e) => {
            error!("Failed to fetch categories: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch categories")
        }
    }
}
