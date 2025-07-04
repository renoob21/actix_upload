use std::{collections::HashMap, env, sync::{Arc, Mutex}};

use actix_cors::Cors;
use actix_files::Files;
use chrono::Local;
use dotenv::dotenv;

use actix_web::{get, http::{self, header::HeaderName}, web, App, HttpResponse, HttpServer, Responder};
use sqlx::{postgres::PgPoolOptions, PgPool};
use utils::models::Session;

mod owner;
mod rent_property;
mod sale_property;
mod utils;
mod user;

#[derive(Clone)]
struct AppState {
    db_pool: PgPool,
    session_store: Arc<Mutex<HashMap<String, Session>>>
}

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
    
    println!("[{}] Server starting up...", Local::now().to_rfc3339());

    let db_url = env::var("DATABASE_URL").expect("Please provide a database url");
    let address = env::var("ADDRESS").expect("Please provide address to bind");
    let port = env::var("PORT").expect("Please provide port to bind");
    // let cors_socket = env::var("CORS_SOCKET").expect("Please provide cors socket");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create pool");

    let shared_state = AppState {
        db_pool: db_pool.clone(),
        session_store: Arc::new(Mutex::new(HashMap::new())),
    };

    let app_state = web::Data::new(shared_state);


    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:5500")
            .allowed_origin("https://renoob21.github.io")
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                http::header::CONTENT_TYPE,
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                HeaderName::from_static("session_id")
            ]).max_age(3600);

        // let cors = Cors::permissive();

        

        App::new()
            .service(hello)
            .service(greetings)
            .app_data(app_state.clone())
            .configure(owner::init_routes)
            .configure(rent_property::init_routes)
            .configure(sale_property::init_routes)
            .configure(user::init_routes)
            .service(Files::new("/rent-pictures", "./uploaded/rents"))
            .service(Files::new("/sale-pictures", "./uploaded/sales"))
            .wrap(cors)
    })
    .bind((address, str::parse::<u16>(&port).unwrap()))?
    .run()
    .await
}