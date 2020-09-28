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

#[derive(Debug, Clone)]
enum Registration {
    Open,
    Invite,
    Closed,
}

#[derive(Debug, Clone)]
struct Server {
    name: String,
    server_url: String,
    logo_url: Option<String>,
    admins: Vec<String>,
    categories: Vec<String>,
    rules: String,
    description: String,
    registration_status: Registration,
}

#[derive(Debug, Clone)]
struct Category {
    name: String,
    servers: Vec<Server>,
}

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

#[get("/details/{server}")]
async fn details_endpoint(web::Path(server): web::Path<String>) -> impl Responder {
    // TODO get server from database
    let current_server = Server {
        name: "Conduit Nordgedanken".into(),
        server_url: "https://conduit.nordgedanken.dev".into(),
        logo_url: None,
        admins: vec!["@mtrnord:conduit.nordgedanken.dev".into()],
        categories: vec!["Test".into(), "test2".into()],
        rules: "Be Nice".into(),
        description: "A conduit Testserver".into(),
        registration_status: Registration::Open,
    };
    DetailsTemplate {
        server: current_server,
    }
    .into_response()
}

#[get("/category/{category}")]
async fn category_endpoint(web::Path(category): web::Path<String>) -> impl Responder {
    // TODO get available categories from database
    // TODO get available servers from database
    let test_category = Category {
        name: "Test".into(),
        servers: vec![Server {
            name: "Conduit Nordgedanken".into(),
            server_url: "https://conduit.nordgedanken.dev".into(),
            logo_url: None,
            admins: vec!["@mtrnord:conduit.nordgedanken.dev".into()],
            categories: vec!["Test".into(), "test2".into()],
            rules: "Be Nice".into(),
            description: "A conduit Testserver".into(),
            registration_status: Registration::Open,
        }],
    };
    let current_category = if category == "Test" {
        test_category.clone()
    } else {
        Category {
            name: "Test2".into(),
            servers: vec![],
        }
    };
    IndexTemplate {
        categories: vec![
            test_category,
            Category {
                name: "Test2".into(),
                servers: vec![],
            },
        ],
        current_category: Some(current_category),
    }
    .into_response()
}

#[get("/")]
async fn index() -> impl Responder {
    // TODO get available categories from database
    IndexTemplate {
        categories: vec![
            Category {
                name: "Test".into(),
                servers: vec![Server {
                    name: "Conduit Nordgedanken".into(),
                    server_url: "https://conduit.nordgedanken.dev".into(),
                    logo_url: None,
                    admins: vec!["@mtrnord:conduit.nordgedanken.dev".into()],
                    categories: vec!["Test".into(), "test2".into()],
                    rules: "Be Nice".into(),
                    description: "A conduit Testserver".into(),
                    registration_status: Registration::Open,
                }],
            },
            Category {
                name: "Test2".into(),
                servers: vec![],
            },
        ],
        current_category: None,
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
            .service(details_endpoint)
            .route("/assets/{filename:.*.css}", web::get().to(css))
            .route("/assets/{filename:.*.js}", web::get().to(js))
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("HOST is not set in .env file");
            let port = env::var("PORT").expect("PORT is not set in .env file");
            println!("Server listening to: {}:{}", host, port);
            server.bind(format!("{}:{}", host, port))?
        }
    };

    println!("Starting server");
    server.run().await?;

    Ok(())
}
