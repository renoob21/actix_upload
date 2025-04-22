use std::env;

use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{get, post, web::{self, ServiceConfig}, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};
use uuid::Uuid;

use crate::utils::{models::ApiResponse, save_uploaded_file};

#[derive(Debug, MultipartForm)]
struct SaleUploadForm {
    #[multipart(rename = "picture")]
    picture: TempFile,
    owner: Text<String>,
    title: Text<String>,
    description: Text<String>,
    address: Text<String>,
    lt: Text<i32>,
    lb: Text<i32>,
    bedroom: Text<i16>,
    bathroom: Text<i16>,
    property_price: Text<i64>
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct SaleProperty {
    sale_property_id: Uuid,
    title: String,
    description: String,
    address: String,
    owner_id: String,
    lt: i32,
    lb: i32,
    bedroom: i16,
    bathroom: i16,
    property_price: i64,
    picture_url: String,
    status: String
}

#[post("/api/sale-property")]
async fn add_sale_property(db_pool: web::Data<PgPool>, mp: MultipartForm<SaleUploadForm>) -> impl Responder {
    let host_url = env::var("HOST_URL").expect("Please provide HOST_URL");
    let picture_name = match mp.picture.file_name.clone() {
        Some(name) => name,
        None => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Uploaded file error".to_string(), None, Some("Error: Unable to get file name".to_string()))
        )
    };

    let file_path = format!("./uploaded/sales/{}", picture_name);
    let url_path = format!("{}/sale-pictures/{}", host_url, picture_name);

    // check extension of file
    match infer::get_from_path(mp.picture.file.path()) {
        Err(err) => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Failed reading uploaded file".to_string(), None, Some(err.to_string()))
        ),
        Ok(path) => match path {
            None => return HttpResponse::BadRequest().json(
                ApiResponse::<()>::new(false, "Unable to read file type".to_string(), None, Some("Error: File type unknown".to_string()))
            ),
            Some(kind) => {
                if (kind.mime_type() == "image/jpeg" && (kind.extension() == "jpg" || kind.extension() == "jpeg")) || (kind.mime_type() == "image/png" && kind.extension() == "png") {
                    println!("Image extension valid");
                } else {
                    return HttpResponse::BadRequest().json(
                        ApiResponse::<()>::new(false, "Invalid file extension".to_string(), None, Some("Invalid file type. Picture must be in [.jpg, .jpeg, .png]".to_string()))
                    );
                }
            }
        }
    }

    if let Err(err) = save_uploaded_file(&mp.picture, &file_path).await {
        return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Failed saving file".to_string(), None, Some(err.to_string()))
        );
    };

    let property_id = Uuid::new_v4();
    let owner_id = match Uuid::parse_str(&mp.owner) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Failed converting owner ID".to_string(), None, Some("Error: Invalid UUID format on owner_id".to_string()))
        )
    };

    let result = sqlx::query_as!(
        SaleProperty,
        "INSERT INTO sale_property(sale_property_id, title, description, address, owner_id, lt, lb, bedroom, bathroom, property_price, picture_url, status)
            VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'Available')
            RETURNING *",
            property_id,
            *mp.title,
            *mp.description,
            *mp.address,
            owner_id,
            *mp.lt,
            *mp.lb,
            *mp.bedroom,
            *mp.bathroom,
            *mp.property_price,
            url_path
    ).fetch_one(db_pool.get_ref()).await;

    match result {
        Ok(property) => HttpResponse::Ok().json(ApiResponse::new(true, "Successfully inserted sale property".to_string(), Some(property), None)),
        Err(err) => HttpResponse::InternalServerError().json(ApiResponse::<()>::new(false, "Failed inserting sale property".to_string(), None, Some(err.to_string())))
    }
}

#[get("/api/sale-property")]
async fn get_sale_properties(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query_as!(
        SaleProperty,
        "SELECT * FROM sale_property"
    ).fetch_all(db_pool.get_ref()).await;

    match result {
        Ok(properties) => HttpResponse::Ok().json(
            ApiResponse::new(true, "Successfully fetch sale property".to_string(), Some(properties), None)
        ),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Failed fetching sale property".to_string(), None, Some("Error: No sale property found".to_string()))
            ),
            _ => HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Failed fetching sale property".to_string(), None, Some(err.to_string()))
            )
        }
    }
}

#[get("/api/sale-property/{sale_property_id}")]
async fn get_sale_property_by_id(db_pool: web::Data<PgPool>, sale_proerty_id: web::Path<Uuid>) -> impl Responder {
    let result = sqlx::query_as!(
        SaleProperty,
        "SELECT * FROM sale_property WHERE sale_property_id = $1",
        *sale_proerty_id
    ).fetch_one(db_pool.get_ref()).await;

    match result {
        Ok(property) => HttpResponse::Ok().json(
            ApiResponse::new(true, "Successfully Retrieved Property".to_string(), Some(property), None)
        ),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Failed Fetching Property".to_string(), None, Some(format!("Error: No property matching id: {}", *sale_proerty_id)))
            ),
            _ => HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Failed to fetch property".to_string(), None, Some(err.to_string()))
            )
        }
    }
    
}

pub fn init_routes(cfg: &mut ServiceConfig) {
    cfg
        .service(add_sale_property)
        .service(get_sale_properties)
        .service(get_sale_property_by_id);
}