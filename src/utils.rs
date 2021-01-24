use crate::errors::ServerError;
use crate::models::MatrixSSWellKnown;
use actix_session::Session;
use actix_web::HttpResponse;
use regex::Regex;
use reqwest::StatusCode;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};

pub enum MatrixSSServername {
    IP(SocketAddr),
    Host(String),
}

/// Resolves the server_name for usage with matrix S-S-Api according too https://matrix.org/docs/spec/server_server/latest#server-discovery
/// It is required that the HOST header always gets set to the server_name when it is returning a IP.
pub async fn resolve_server_name(server_name: String) -> Result<MatrixSSServername, ServerError> {
    // If ip literal with port
    if let Ok(addr) = SocketAddr::from_str(&server_name) {
        return Ok(MatrixSSServername::IP(addr));
    } else if let Ok(ip) = IpAddr::from_str(&server_name) {
        // If ip without port
        return Ok(MatrixSSServername::IP(SocketAddr::new(ip, 8448)));
    }

    let resolver = TokioAsyncResolver::tokio(ResolverConfig::quad9_tls(), ResolverOpts::default())
        .map_err(|_| ServerError::MatrixFederationWronglyConfigured)?;

    // If has hostname and port
    let port_re = Regex::new(r":([0-9]+)$").unwrap();
    if port_re.is_match(&server_name) {
        let caps = port_re.captures(&server_name).unwrap();
        let hostname = port_re.replace(&server_name, "");
        let port = caps.get(1).unwrap().as_str();

        // Get AAAA/A record
        let results = resolver
            .lookup_ip(hostname.to_string())
            .await
            .map_err(|_| ServerError::MatrixFederationWronglyConfigured)?;

        let ip: IpAddr = results
            .iter()
            .next()
            .ok_or(ServerError::MatrixFederationWronglyConfigured)?;
        return Ok(MatrixSSServername::IP(SocketAddr::new(
            ip,
            port.parse().unwrap(),
        )));
    }

    // Check well-known file
    let resp = reqwest::get(&format!(
        "https://{}/.well-known/matrix/server",
        server_name
    ))
    .await;

    if let Ok(resp) = resp {
        if resp.status() == StatusCode::OK {
            let body: MatrixSSWellKnown = resp
                .json::<MatrixSSWellKnown>()
                .await
                .map_err(|_| ServerError::MatrixFederationWronglyConfigured)?;

            return if port_re.is_match(&body.server) {
                let caps = port_re.captures(&body.server).unwrap();
                let hostname = port_re.replace(&body.server, "");
                let port = caps.get(1).unwrap().as_str();
                Ok(MatrixSSServername::Host(format!("{}:{}", hostname, port)))
            } else {
                // FIXME if we have a hostname and no port we actually shall check SRV first
                Ok(MatrixSSServername::Host(format!("{}:8448", body.server)))
            };
        }
    }

    // Check SRV record
    let results = resolver
        .srv_lookup(format!("_matrix._tcp.{}", server_name))
        .await;
    if let Ok(results) = results {
        let first = results
            .iter()
            .next()
            .ok_or(ServerError::MatrixFederationWronglyConfigured)?;
        let target = first.target().to_string();
        let host = target.trim_end_matches('.');
        let port = first.port();
        return Ok(MatrixSSServername::IP(SocketAddr::new(
            IpAddr::from_str(host).map_err(|_| ServerError::MatrixFederationWronglyConfigured)?,
            port,
        )));
    }
    Ok(MatrixSSServername::Host(format!("{}:8448", server_name)))
}

pub fn check_logged_in(session: &Session) -> Option<HttpResponse> {
    if let Ok(Some(_)) = session.get::<String>("mxid") {
        if let Ok(Some(_)) = session.get::<String>("server") {
            return Some(HttpResponse::Ok().body("success"));
        }
    }
    None
}
