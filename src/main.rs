#![allow(dead_code)]
#![allow(clippy::async_yields_async)] // Actix-web makes issues

use crate::errors::ServerError;
use crate::models::{Category, CategoryDB, OauthResponse, Registration, Server};
use actix_files::NamedFile;
use actix_web::web::Query;
use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use askama_actix::{Template, TemplateIntoResponse};
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

#[derive(Template, Debug)]
#[template(path = "index.html")]
struct IndexTemplate {
    categories: Vec<Category>,
    current_category: Option<Category>,
}

#[derive(Template, Debug)]
#[template(path = "details.html")]
struct DetailsTemplate {
    server: Server,
}

#[derive(Template, Debug)]
#[template(path = "oauth_error.html")]
struct OAuthErrorTemplate {
    error: OauthResponse,
}

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
#[get("/oauth/done")]
async fn oauth_done(
    db_pool: web::Data<PgPool>,
    Query(oauth_resp): Query<OauthResponse>,
) -> impl Responder {
    if oauth_resp.code.is_none() && oauth_resp.error.is_none() {
        let error = OauthResponse {
            code: None,
            error: Some("invalid_request".into()),
            error_description: Some("OAuth server did not return a code".into()),
        };
        let template_result = OAuthErrorTemplate { error }.into_response();
        return match template_result {
            Ok(r) => r,
            Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        };
    }
    if oauth_resp.error.is_some() {
        let template_result = OAuthErrorTemplate { error: oauth_resp }.into_response();
        return match template_result {
            Ok(r) => r,
            Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        };
    }

    // FIXME use some config vars and cycle secret
    let code = oauth_resp.code.clone().unwrap();
    let params = [
        ("redirect_uri", "https://joinmatrix.rocks/admin"),
        ("client_id", "keymaker"),
        ("client_secret", "keymaker-secret"),
        ("code", &code),
        ("grant_type", "authorization_code"),
    ];
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth.joinmatrix.rocks/oauth/token")
        .form(&params)
        .send()
        .await;
    match resp {
        Ok(resp) => {
            if resp.status() == StatusCode::OK {
                // TODO redirect to admin page
                // TODO Session handling
                return HttpResponse::Ok().body("Successful OAuth flow");
            } else {
                let status = resp.status();
                let bytes = resp.bytes().await.unwrap();
                let body = String::from_utf8_lossy(&bytes);
                tracing::error!(
                    "Unexpected non 200 Code. Code was: {}. Resp Body was: {:?}",
                    status,
                    body
                );
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error while getting resp: {}", e);
        }
    }
    HttpResponse::Unauthorized().body("Please try again!")
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
            .service(index)
            .service(category_endpoint)
            .service(servers)
            .service(details_endpoint)
            .service(oauth_done)
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
