use actix_web::{FromRequest, HttpRequest, dev::Payload, web};

use crate::DOMAIN;

pub struct MaybeCustomClient {
    pub is_custom: bool,
    pub cookies_header: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CookieQuery {
    cookie: Option<String>,
}

impl FromRequest for MaybeCustomClient {
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // get query param
            let query = web::Query::<CookieQuery>::from_query(req.query_string())
                .ok()
                .and_then(|q| q.into_inner().cookie);

            if let Some(cookie_string) = query {
                // user provided custom cookie header; keep it to return to handlers
                Ok(MaybeCustomClient {
                    is_custom: true,
                    cookies_header: Some(cookie_string),
                })
            } else {
                Ok(MaybeCustomClient {
                    is_custom: false,
                    cookies_header: None,
                })
            }
        })
    }
}
