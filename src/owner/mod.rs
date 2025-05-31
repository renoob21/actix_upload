use actix_web::{get, web::{self, ServiceConfig}, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::Uuid};

use crate::{utils::models::ApiResponse, AppState};

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Owner {
    owner_id: Uuid,
    owner_name: String,
    address: String,
    email: String
}

#[get("/api/owner")]
async fn get_owners(app_state: web::Data<AppState>) -> impl Responder {
    let result = sqlx::query_as!(
        Owner,
        "SELECT * FROM property_owner"
    ).fetch_all(&app_state.db_pool).await;


    match result {
        Ok(owners) => HttpResponse::Ok().json(
            ApiResponse::new(true, "Successfully fetched owner list".to_string(), Some(owners), None)
        ),
        Err(err) => HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Failed fetching owner list".to_string(), None, Some(err.to_string()))
        )
    }
    
}

#[get("/api/owner/{owner_id}")]
async fn get_owner_by_id(app_state: web::Data<AppState>, owner_id: web::Path<Uuid>) -> impl Responder {
    let result = sqlx::query_as!(
        Owner,
        "SELECT * FROM property_owner WHERE owner_id = $1",
        owner_id.into_inner()
    ).fetch_one(&app_state.db_pool).await;

    match result {
        Ok(owner) => HttpResponse::Ok().json(
            ApiResponse::new(true, "Successfully fetched owner data".to_string(), Some(owner), None)
        ),
        Err(err) => HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Failed fetching owner data".to_string(), None, Some(err.to_string()))
        )
    }
}

pub fn init_routes(cfg: &mut ServiceConfig) {
    cfg
        .service(get_owners)
        .service(get_owner_by_id);
}