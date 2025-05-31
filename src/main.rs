use std::{collections::HashMap, env, sync::{Arc, Mutex}};

use actix_cors::Cors;
use actix_files::Files;
use chrono::Local;
use dotenv::dotenv;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use sqlx::{postgres::PgPoolOptions, PgPool};
use utils::models::Session;
use uuid::Uuid;

mod owner;
mod rent_property;
mod sale_property;
mod utils;
mod user;

#[derive(Clone)]
struct AppState {
    instance_id: Uuid,
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
    let app_instance_id = Uuid::new_v4(); // Generate a unique ID for this server run
    println!("[{}] Server starting up... AppState Instance ID will be: {}", Local::now().to_rfc3339(), app_instance_id);

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
        instance_id: Uuid::new_v4(),
    };

    let app_state = web::Data::new(shared_state);


    HttpServer::new(move || {
        // let cors = Cors::default()
        //     .allowed_origin(&cors_socket)
        //     .allow_any_method()
        //     .allow_any_header();

        let cors = Cors::permissive();

        

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