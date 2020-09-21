#![allow(dead_code)]

use actix_files::NamedFile;
use actix_web::{
    get, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
    Result as ActixResult,
};
use anyhow::Result;
use askama_actix::{Template, TemplateIntoResponse};
use dotenv::dotenv;
use listenfd::ListenFd;
use sqlx::PgPool;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Template, Debug)]
#[template(path = "index.html")]
struct IndexTemplate {
    // TODO use Vec of Category
    categories: Vec<String>,
    current_category: Option<String>,
    // TODO use Vec of Servers
    servers: Vec<String>,
}

#[get("/category/{category}")]
async fn category_endpoint(web::Path(category): web::Path<String>) -> impl Responder {
    // TODO get available categories from database
    // TODO get available servers from database
    IndexTemplate {
        categories: vec!["Test".into(), "test2".into()],
        current_category: Some(category),
        servers: vec![],
    }
    .into_response()
}

#[get("/")]
async fn index() -> impl Responder {
    // TODO get available categories from database
    IndexTemplate {
        categories: vec!["Test".into(), "test2".into()],
        current_category: None,
        servers: vec![],
    }
    .into_response()
}

#[get("/api/servers")]
async fn servers() -> impl Responder {
    HttpResponse::Ok().body("{}")
}

async fn css(req: HttpRequest) -> ActixResult<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    let real_path = Path::new("assets/css/").join(path);
    Ok(NamedFile::open(real_path)?)
}

async fn js(req: HttpRequest) -> ActixResult<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    let real_path = Path::new("assets/js/").join(path);
    Ok(NamedFile::open(real_path)?)
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // this will enable us to keep application running during recompile: systemfd --no-pid -s http::5000 -- cargo watch -x run
    let mut listenfd = ListenFd::from_env();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db_pool = PgPool::new(&database_url).await?;

    let mut server = HttpServer::new(move || {
        App::new()
            .data(db_pool.clone()) // pass database pool to application so we can access it inside handlers
            .wrap(middleware::Compress::default())
            .service(index)
            .service(category_endpoint)
            .service(servers)
            .route("/assets/{filename:.*.css}", web::get().to(css))
            .route("/assets/{filename:.*.js}", web::get().to(js))
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("HOST is not set in .env file");
            let port = env::var("PORT").expect("PORT is not set in .env file");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    println!("Starting server");
    server.run().await?;

    Ok(())
}
