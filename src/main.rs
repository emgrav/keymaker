#![allow(dead_code)]
#![allow(clippy::async_yields_async)] // Actix-web makes issues

use crate::errors::ServerError;
use crate::models::{CategoryDB, LoginData, Registration, Server};
use crate::templates::{DetailsTemplate, IndexTemplate, LoginTemplate};
use actix_files::NamedFile;
use actix_session::{CookieSession, Session};
use actix_web::web::Query;
use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use askama_actix::TemplateIntoResponse;
use color_eyre::Result;
use dotenv::dotenv;
use listenfd::ListenFd;
use reqwest::StatusCode;
use sqlx::PgPool;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tracing::{info, instrument, Level};

mod errors;
mod models;
mod templates;
mod utils;

#[instrument]
#[get("/details/{server_url}")]
async fn details_endpoint(web::Path(server_url): web::Path<String>) -> impl Responder {
    // TODO get server from database
    let current_server = Server {
        name: "Conduit Nordgedanken".into(),
        url: "https://conduit.nordgedanken.dev".into(),
        server_name: "nordgedanken.dev".into(),
        logo_url: None,
        admins: vec!["@mtrnord:conduit.nordgedanken.dev".into()],
        categories: vec![],
        rules: "Be Nice".into(),
        description: "A conduit Testserver".into(),
        registration_status: Registration::Open,
    };
    DetailsTemplate {
        server: current_server,
    }
    .into_response()
}

#[instrument]
#[get("/category/{category_name}")]
async fn category_endpoint(
    web::Path(category_name): web::Path<String>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    let result = CategoryDB::get_all(db_pool.get_ref()).await;
    match result {
        Ok(categories) => {
            let current_category = categories
                .clone()
                .into_iter()
                .find(|category| category.name == category_name);
            let template_result = IndexTemplate {
                categories,
                current_category,
            }
            .into_response();
            match template_result {
                Ok(r) => r,
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        _ => HttpResponse::InternalServerError().body("Failed to load categories"),
    }
}

#[instrument]
#[get("/")]
async fn index(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = CategoryDB::get_all(db_pool.get_ref()).await;
    match result {
        Ok(categories) => {
            let template_result = IndexTemplate {
                categories,
                current_category: None,
            }
            .into_response();
            match template_result {
                Ok(r) => r,
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        _ => HttpResponse::InternalServerError().body("Failed to load categories"),
    }
}

#[instrument]
#[get("/auth/login")]
async fn auth_login() -> impl Responder {
    LoginTemplate {}.into_response()
}

#[instrument(skip(session))]
#[get("/auth/done")]
async fn auth_done(session: Session, Query(login_data): Query<LoginData>) -> impl Responder {
    if let Ok(Some(mxid)) = session.get::<String>("mxid") {
        if let Ok(Some(server)) = session.get::<String>("server") {
            if mxid == login_data.mxid && server == login_data.server {
                // TODO redirect to admin panel
            }
        }
    }

    // TODO use utils::resolve_server_name
    // TODO Do https://matrix.org/docs/spec/server_server/latest#get-matrix-federation-v1-openid-userinfo
    // TODO check if sub and mxid match
    // TODO write session cookie
    // TODO redirect to admin interface

    HttpResponse::Ok().body("success")
}

#[instrument]
#[get("/api/servers")]
async fn servers() -> impl Responder {
    HttpResponse::Ok().body("{}")
}

#[instrument]
async fn css(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    if path.extension().and_then(OsStr::to_str) != Some("css") {
        return Err(ServerError::PathTraversal.into());
    }
    let real_path = Path::new("assets/css/").join(path);
    Ok(NamedFile::open(real_path)?)
}

#[instrument]
async fn js(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    if path.extension().and_then(OsStr::to_str) != Some("js") {
        return Err(ServerError::PathTraversal.into());
    }
    let real_path = Path::new("assets/js/").join(path);
    Ok(NamedFile::open(real_path)?)
}

#[instrument]
#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        // all spans/events with a level higher than DEBUG (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // sets this to be the default, global subscriber for this application.
        .init();
    dotenv().ok();

    // this will enable us to keep application running during recompile: systemfd --no-pid -s http::5000 -- cargo watch -x run
    let mut listenfd = ListenFd::from_env();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db_pool = PgPool::connect(&database_url).await?;

    let mut server = HttpServer::new(move || {
        App::new()
            .data(db_pool.clone()) // pass database pool to application so we can access it inside handlers
            .wrap(middleware::Compress::default())
            .wrap(
                CookieSession::private(&[0; 32]) // <- create cookie based session middleware
                    .secure(false),
            )
            .service(index)
            .service(category_endpoint)
            .service(servers)
            .service(details_endpoint)
            .service(auth_login)
            .service(auth_done)
            .route("/assets/{filename:.*.css}", web::get().to(css))
            .route("/assets/{filename:.*.js}", web::get().to(js))
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("HOST is not set in .env file");
            let port = env::var("PORT").expect("PORT is not set in .env file");
            info!("Server listening to: {}:{}", host, port);
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await?;

    Ok(())
}
