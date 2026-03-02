use std::net::SocketAddr;
use trust_dns_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};

pub struct AsyncDNSResolverAdapter {
    system_resolver: TokioAsyncResolver,
    cloudflare_resolver: TokioAsyncResolver,
}

impl AsyncDNSResolverAdapter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let system_resolver = TokioAsyncResolver::tokio_from_system_conf()?;
        let cloudflare_resolver = TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());

        Ok(AsyncDNSResolverAdapter {
            system_resolver,
            cloudflare_resolver,
        })
    }

    pub async fn lookup_ip(&self, domain: &str) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
        match self.system_resolver.lookup_ip(domain).await {
            Ok(lookup) => Ok(lookup.iter().map(|ip| SocketAddr::new(ip, 443)).collect()),
            Err(_) => match self.cloudflare_resolver.lookup_ip(domain).await {
                Ok(lookup) => Ok(lookup.iter().map(|ip| SocketAddr::new(ip, 443)).collect()),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            },
        }
    }
}
