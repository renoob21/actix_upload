use std::env;

use actix_cors::Cors;
use actix_files::Files;
use dotenv::dotenv;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use sqlx::postgres::PgPoolOptions;

mod owner;
mod rent_property;
mod sale_property;
mod utils;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello")
}

#[get("/greetings/{name}")]
async fn greetings(name: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello, {}", name))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("Please provide a database url");
    let address = env::var("ADDRESS").expect("Please provide address to bind");
    let port = env::var("PORT").expect("Please provide port to bind");
    let cors_socket = env::var("CORS_SOCKET").expect("Please provide cors socket");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create pool");


    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:5500")
            .allow_any_method()
            .allow_any_header();


        App::new()
            .service(hello)
            .service(greetings)
            .app_data(web::Data::new(db_pool.clone()))
            .configure(owner::init_routes)
            .configure(rent_property::init_routes)
            .configure(sale_property::init_routes)
            .service(Files::new("/rent-pictures", "./uploaded/rents"))
            .wrap(cors)
    })
    .bind((address, str::parse::<u16>(&port).unwrap()))?
    .run()
    .await
}