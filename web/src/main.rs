#![warn(clippy::all)]

use std::env;
use std::error::Error;
use std::io;

use actix_files;
use actix_web::{http::ContentEncoding, middleware, web, App, HttpServer};
use dotenv::dotenv;
use handlebars::Handlebars;

use db::{build_pool, establish_connection, run_migrations, SqliteConnectionPool};

use crate::controllers::{api, view};

/// Represents the [server state](actix_web.ServerState.html) for the application.
pub struct ServerData {
    pub db: SqliteConnectionPool,
    pub template: Handlebars,
}

/// Registers the [Handlebars](handlebars.handlebars.html) templates for the application.
fn register_templates() -> Result<Handlebars, Box<dyn Error>> {
    let mut tpl = Handlebars::new();
    tpl.set_strict_mode(true);
    tpl.register_templates_directory(".hbs", "./web/templates/")?;

    Ok(tpl)
}

fn main() -> io::Result<()> {
    dotenv().ok();

    // Set up logging
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Run DB migrations for a new SQLite database
    run_migrations(&establish_connection(&url)).expect("Error running migrations");

    let pool = build_pool(&url);

    HttpServer::new(move || {
        // Create handlebars registry
        let template = register_templates().unwrap();

        // Wire up the application
        App::new()
            .wrap(middleware::Compress::new(ContentEncoding::Gzip))
            .wrap(middleware::Logger::default())
            .data(ServerData {
                db: pool.clone(),
                template,
            })
            .service(actix_files::Files::new("/static", "./web/dist").use_etag(true))
            .service(web::resource("about").to(view::about))
            .service(
                web::resource("/")
                    .name("bible")
                    .route(web::get().to_async(view::all_books)),
            )
            .service(web::resource("search").route(web::get().to_async(view::search)))
            .service(
                web::resource("{book}")
                    .name("book")
                    .route(web::get().to_async(view::book)),
            )
            .service(
                web::resource("{reference:.+\\d}")
                    .name("reference")
                    .route(web::get().to_async(view::reference)),
            )
            .service(web::resource("api/search").route(web::get().to_async(api::search)))
            .service(
                web::resource("api/{reference}.json").route(web::get().to_async(api::reference)),
            )
            .default_service(web::route().to(web::HttpResponse::NotFound))
    })
    .bind("0.0.0.0:8080")?
    .run()
}

mod controllers;
mod error;
mod json_ld;
