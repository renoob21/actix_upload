use std::env;

use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{get, post, web::{self, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use chrono::{NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slug::slugify;
use sqlx::{prelude::FromRow};
use uuid::Uuid;

use crate::{utils::models::ApiResponse, AppState};

use super::utils::save_uploaded_file;

#[derive(Debug, MultipartForm)]
struct RentUploadForm {
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
    monthly_rent: Text<i64>
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct RentProperty {
    pub rent_property_id: Uuid,
    pub title: String,
    pub description: String,
    pub address: String,
    pub owner_id: String,
    pub lt: i32,
    pub lb: i32,
    pub bedroom: i16,
    pub bathroom: i16,
    pub monthly_rent: i64,
    pub picture_url: String,
    pub status: String
}

#[post("/api/rent-property")]
async fn add_rent_property(app_state: web::Data<AppState>, mp: MultipartForm<RentUploadForm>) -> impl Responder {
    let host_address = env::var("HOST_URL").expect("Please provide HOST URL");
    let picture_name = match mp.picture.file_name.clone() {
        Some(name) => {
            let file_path : Vec<&str> = name.split(".").collect();
            format!("{}.{}", slugify(file_path[0]), file_path[file_path.len() - 1])
        },
        None => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Uploaded file error".to_string(), None, Some("Error: Unable to get file name".to_string()))
        )
    };
    let file_path = format!("./uploaded/rents/{}", picture_name);
    let url_path = format!("{}/rent-pictures/{}", host_address, picture_name);

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
    }

    let property_id = Uuid::new_v4();
    let owner_id = match Uuid::parse_str(&mp.owner) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Failed converting owner ID".to_string(), None, Some("Error: Invalid UUID format on owner_id".to_string()))
        )
    };

    let result = sqlx::query_as!(
        RentProperty,
        "INSERT INTO rent_property(rent_property_id, title, description, address, owner_id, lt, lb, bedroom, bathroom, monthly_rent, picture_url, status)
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
            *mp.monthly_rent,
            url_path
    ).fetch_one(&app_state.db_pool).await;


    match result {
        Err(err) => HttpResponse::InternalServerError().json(ApiResponse::<()>::new(false, "Failed inserting new rent_property".to_string(), None, Some(err.to_string()))),
        Ok(property) => HttpResponse::Ok().json(ApiResponse::new(true, "Successfully insert new property".to_string(), Some(property), None))
    }
}



#[get("/api/rent-property")]
async fn get_rent_properties(app_state: web::Data<AppState>) -> impl Responder {
    let result = sqlx::query_as!(
        RentProperty,
        "SELECT * FROM rent_property WHERE rent_property_id not in 
        (
        SELECT rent_property_id FROM rent_transaction
        WHERE status != 'Cancelled' AND end_date > CURRENT_DATE
        )"
    ).fetch_all(&app_state.db_pool).await;

    match result {
     Ok(properties) => HttpResponse::Ok().json(
        ApiResponse::new(true, "Successfully retrieved rental properties".to_string(), Some(properties), None)
     ),
     Err(_) => HttpResponse::InternalServerError().json(
        ApiResponse::<()>::new(false, "Failed retrieving rental properties".to_string(), None, Some("Server unable to retrieve data".to_string()))
     )
    }
}

#[get("/api/rent-property/{rent_property_id}")]
async fn get_rent_property_by_id(app_state: web::Data<AppState>, rent_property_id: web::Path<Uuid>) -> impl Responder {
    let result = sqlx::query_as!(
        RentProperty,
        "SELECT * FROM rent_property WHERE rent_property_id = $1 
        AND rent_property_id not in 
        (
        SELECT rent_property_id FROM rent_transaction
        WHERE status != 'Cancelled' AND end_date > CURRENT_DATE
        )",
        *rent_property_id
    ).fetch_one(&app_state.db_pool).await;

    match result {
        Ok(property) => HttpResponse::Ok().json(ApiResponse::new(true,"Successfully retrieved property".to_string(), Some(property), None)),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Property not found".to_string(), None, Some(format!("Error: No property matching id: {}", *rent_property_id)))
            ),
            _ => HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Unable to retrieve property".to_string(), None, Some(err.to_string()))
            )
        }
    }
}



use crate::{utils::{get_session}};

#[derive(Debug, Deserialize, Serialize, FromRow)]
struct RentTransaction {
    rent_transaction_id: Uuid,
    rent_property_id: Uuid,
    user_id: Uuid,
    total_payment: Option<i64>,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    status: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
struct RentTransactionObject {
    rent_transaction_id: Uuid,
    user_id: Uuid,
    total_payment: Option<i64>,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    status: String,
    rent_property: Value
}

#[derive(Debug, Deserialize)]
struct RentTransactionForm {
    rent_property_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[post("/api/rent-transaction")]
async fn post_rent_transaction(app_state: web::Data<AppState>, req: HttpRequest, rent_form: web::Json<RentTransactionForm>) -> impl Responder {
    let user_session = match get_session(app_state.clone(), &req).await {
        Err(err) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user session".to_string(), None, Some(err))
        ),
        Ok(session) => session,
    };

    let new_transaction_id = Uuid::new_v4();

    let property = match sqlx::query_as!(
        RentProperty,
        "SELECT * FROM rent_property WHERE rent_property_id = $1
        AND rent_property_id not in 
        (
        SELECT rent_property_id FROM rent_transaction
        WHERE status != 'Cancelled' AND end_date > CURRENT_DATE
        )",
        rent_form.rent_property_id
    ).fetch_one(&app_state.db_pool).await {
        Err(err) => match err {
            sqlx::Error::RowNotFound => return  HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Property not found".to_string(), None, Some(format!("Error: No property matching id: {}", rent_form.rent_property_id)))
            ),
            _ => return  HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Unable to retrieve property".to_string(), None, Some(err.to_string()))
            )
        },
        Ok(prop) => prop
    };

    let rent_length = rent_form.end_date - rent_form.start_date;

    let total_payment = property.monthly_rent * (rent_length.num_days() / 30);

    let result = sqlx::query_as!(
        RentTransaction,
        "INSERT INTO rent_transaction(rent_transaction_id, rent_property_id, user_id, total_payment, start_date, end_date, status)
        VALUES($1, $2, $3, $4, $5, $6, \'Unpaid\') RETURNING *",
        new_transaction_id,
        property.rent_property_id,
        user_session.user_data.user_id,
        total_payment,
        rent_form.start_date,
        rent_form.end_date,
    ).fetch_one(&app_state.db_pool).await;

    match result {
        Err(_) => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false,"Failed submitting form".to_string(), None, Some("Server unable to process form submission".to_string()))
        ),
        Ok(rent) => return  HttpResponse::Ok().json(
            ApiResponse::new(true, "Form submission success".to_string(), Some(rent), None)
        )
    }


    
}

#[get("/api/rent-transaction/{rent_transaction_id}")]
async fn get_rent_transaction_by_id(app_state: web::Data<AppState>, req: HttpRequest, rent_transaction_id: web::Path<Uuid>) -> impl Responder {
    let user_session = match get_session(app_state.clone(), &req).await {
        Err(err) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user session".to_string(), None, Some(err))
        ),
        Ok(session) => session,
    };

    let result = sqlx::query_as!(
        RentTransactionObject,
        "SELECT 
        rt.rent_transaction_id,
        rt.user_id,
        rt.total_payment,
        rt.start_date,
        rt.end_date,
        rt.status,
        JSON_BUILD_OBJECT(
            'rent_property_id', rp.rent_property_id,
            'title', rp.title,
            'description', rp.description,
            'address', rp.address,
            'owner_id', rp.owner_id,
            'lt', rp.lt,
            'lb', rp.lb,
            'bedroom', rp.bedroom,
            'bathroom', rp.bathroom,
            'monthly_rent', rp.monthly_rent,
            'picture_url', rp.picture_url,
            'status', rp.status
        ) AS rent_property
        FROM rent_transaction rt
        JOIN rent_property rp ON rt.rent_property_id = rp.rent_property_id
        WHERE rent_transaction_id = $1;
        ",
        *rent_transaction_id
    ).fetch_one(&app_state.db_pool).await;

    match result {
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Transaction not found".to_string(), None, Some(format!("Error: No transaction matching id: {}", *rent_transaction_id)))
            ),
            _ => HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Unable to retrieve transaction".to_string(), None, Some(err.to_string()))
            )
        },
        Ok(transaction) => {
            if transaction.user_id == user_session.user_data.user_id {
                HttpResponse::Ok().json(
                   ApiResponse::new(true, "Successfully retrieved transaction".to_string(), Some(transaction), None)
                )
            } else {
                HttpResponse::Forbidden().json(
                    ApiResponse::<()>::new(false, "Content restricted".to_string(), None, Some("User does not match transaction owner".to_string()))
                )
            }
        }
    }


    
}

#[get("/api/my-rent-transaction")]
async fn get_my_rent_transaction(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let user_session = match get_session(app_state.clone(), &req).await {
        Err(err) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user session".to_string(), None, Some(err))
        ),
        Ok(session) => session,
    };

    let result = sqlx::query_as!(
        RentTransactionObject,
        "SELECT 
        rt.rent_transaction_id,
        rt.user_id,
        rt.total_payment,
        rt.start_date,
        rt.end_date,
        rt.status,
        JSON_BUILD_OBJECT(
            'rent_property_id', rp.rent_property_id,
            'title', rp.title,
            'description', rp.description,
            'address', rp.address,
            'owner_id', rp.owner_id,
            'lt', rp.lt,
            'lb', rp.lb,
            'bedroom', rp.bedroom,
            'bathroom', rp.bathroom,
            'monthly_rent', rp.monthly_rent,
            'picture_url', rp.picture_url,
            'status', rp.status
        ) AS rent_property
        FROM rent_transaction rt
        JOIN rent_property rp ON rt.rent_property_id = rp.rent_property_id
        WHERE user_id = $1;
        ",
        user_session.user_data.user_id,
    ).fetch_all(&app_state.db_pool).await;


    match result {
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().json(
                ApiResponse::<()>::new(false, "Transaction not found".to_string(), None, Some(format!("Error: No transaction for user id: {}", user_session.user_data.user_id)))
            ),
            _ => HttpResponse::InternalServerError().json(
                ApiResponse::<()>::new(false, "Unable to retrieve transaction".to_string(), None, Some(err.to_string()))
            )
        },
        Ok(transaction) => {
            HttpResponse::Ok().json(
                   ApiResponse::new(true, "Successfully retrieved transaction".to_string(), Some(transaction), None)
                )
        }
    }
    
    
}


#[post("/api/pay-rent/{rent_transaction_id}")]
async fn pay_rent(app_state: web::Data<AppState>, req: HttpRequest, rent_transaction_id: web::Path<Uuid>) -> impl Responder {
    let user_session = match get_session(app_state.clone(), &req).await {
        Err(err) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user session".to_string(), None, Some(err))
        ),
        Ok(session) => session,
    };

    let mut trx = match app_state.db_pool.begin().await {
        Err(err) => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Internal Server Error".to_string(), None, Some(err.to_string()))
        ),

        Ok(tr) => tr
    };

    let result = sqlx::query_as!(RentTransaction,
        "UPDATE rent_transaction SET status = 'Paid' WHERE rent_transaction_id = $1 returning *",
        *rent_transaction_id
    ).fetch_one(&mut *trx).await;

    let rent_transaction = match result {
        Err(err) => match err {
        sqlx::Error::RowNotFound => return  HttpResponse::NotFound().json(
            ApiResponse::<()>::new(false, "Failed processing payment".to_string(), None, Some("Invalid transaction id".to_string()))
        ),
        _ => return  HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Failed processing payment".to_string(), None, Some(err.to_string()))
        )
        },
        Ok(trans) => trans,
    };

    if rent_transaction.user_id != user_session.user_data.user_id {
        trx.rollback().await.unwrap();

        return HttpResponse::Forbidden().json(
            ApiResponse::<()>::new(false, "Failed processing payment".to_string(), None, Some("User not authorized".to_string()))
        );
    }

    trx.commit().await.unwrap();

    HttpResponse::Ok().json(
        ApiResponse::new(true, "Payment processing success".to_string(), Some(rent_transaction), None)
    )
}

pub fn init_routes(cfg: &mut ServiceConfig) {
    cfg
        .service(add_rent_property)
        .service(get_rent_properties)
        .service(get_rent_property_by_id)
        .service(post_rent_transaction)
        .service(get_rent_transaction_by_id)
        .service(get_my_rent_transaction)
        .service(pay_rent);
}