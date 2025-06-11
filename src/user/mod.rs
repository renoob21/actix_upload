use actix_web::{get, post, web::{self, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use bcrypt::{hash, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as, PgPool, Result};
use uuid::Uuid;

use crate::{utils::{get_session, models::{ApiResponse, Session}}, AppState};

#[derive(Debug, Serialize, Deserialize)]
struct UserRegistration {
    full_name : String,
    email_address: String,
    address: String,
    password: String
}

#[derive(Debug, Deserialize)]
struct UserLogin {
    email_address: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
struct User {
    user_id: Uuid,
    full_name: String,
    email_address: String,
    address: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserData {
    pub user_id: Uuid,
    full_name: String,
    email_address: String,
    address: String
}

impl From<User> for UserData {
    fn from(user: User) -> Self {
        UserData { user_id: user.user_id, full_name: user.full_name, email_address: user.email_address, address: user.address }
    }
}

#[post("/api/user")]
async fn register_user(app_state: web::Data<AppState>, user_form: web::Json<UserRegistration>) -> impl Responder {
    println!("{:?}", user_form);

    let pass_hash = match hash(user_form.password.clone(), DEFAULT_COST) {
        Ok(hash_val) => hash_val,
        Err(_) => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Error inserting user data".to_string(), None, Some("Unable to insert user data".to_string()))
        )
    };
    let user_id = Uuid::new_v4();

    


    let result = query_as!(
        UserData,
        "INSERT INTO \"user\"(user_id, full_name, email_address, address, password)
            VALUES ($1, $2, $3, $4, $5) RETURNING user_id, full_name, email_address, address",
        user_id,
        user_form.full_name,
        user_form.email_address,
        user_form.address,
        pass_hash
    ).fetch_one(&app_state.db_pool).await;

    match result {
        Err(err) => match err {
            sqlx::Error::Database(db_err) => {
                match db_err.kind() {
                    sqlx::error::ErrorKind::UniqueViolation => return HttpResponse::BadRequest().json(
                        ApiResponse::<()>::new(false, "User already exists".to_string(), None, Some(format!("Email: {} already registered", user_form.email_address)))
                    ),
                    _ => (),
                }
            },
            _ => ()

        },
        Ok(data) => return HttpResponse::Ok().json(
            ApiResponse::new(true, "Successfully register new user".to_string(), Some(data), None)
        )
    }

    HttpResponse::InternalServerError().json(
        ApiResponse::<()>::new(false, "Unable to register user".to_string(), None, Some("Unable to register user".to_string()))
    )
}



async fn get_user_by_email(db_pool: &PgPool, email: &str) -> Result<User> {
    let res = query_as!(
        User,
        "SELECT * FROM \"user\" WHERE email_address = $1",
        email
    ).fetch_one(db_pool).await?;

    Ok(res)
}

#[post("/api/login")]
async fn login(app_state: web::Data<AppState>, user_login: web::Json<UserLogin>) -> impl Responder {
    let result = get_user_by_email(&app_state.db_pool, &user_login.email_address).await;


    let user = match result {
        Err(err) => match err {
            sqlx::Error::RowNotFound => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Login Failed".to_string(), None, Some("Incorrect email or password".to_string()))
        ),

        _ => return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user data".to_string(), None, Some("Internal server error occured".to_string()))
        )
        },

        Ok(user) => user
    };

    let is_correct_password = match bcrypt::verify(&user_login.password, &user.password.trim()) {
        Err(err) => {
            println!("{:?}", err);
            return HttpResponse::InternalServerError().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user data".to_string(), None, Some("Internal server error occured".to_string()))
        )},

        Ok(ver) => ver
    };

    if !is_correct_password {
        return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Login Failed".to_string(), None, Some("Incorrect email or password".to_string()))
        )
    }

    let session_id = Uuid::new_v4();

    let new_session = Session::new(session_id, UserData::from(user));


    let mut store_guard = app_state.session_store.lock().unwrap();
    store_guard.insert(session_id.to_string(), new_session.clone());


    HttpResponse::Ok().json(
        ApiResponse::new(true, "Login successful".to_string(), Some(new_session), None)
    )
}

#[get("/api/profile")]
async fn get_profile(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let user_session = match get_session(app_state, &req).await {
        Err(err) => return HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve user session".to_string(), None, Some(err))
        ),
        Ok(session) => session,
    };


    HttpResponse::Ok().json(
        ApiResponse::new(true, "Session retrieve successful".to_string(), Some(user_session), None)
    )
}

#[get("/api/logout")]
async fn logout(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let session_id = match req.headers().get("session_id") {
        None => return  HttpResponse::BadRequest().json(
            ApiResponse::<()>::new(false, "Unable to retrieve session data".to_string(), None, Some("Requires Header: \'session_id\'".to_string()))
        ),
        Some(id) => id.to_str().unwrap()
    };

    let mut session_store = app_state.session_store.lock().unwrap();

    session_store.remove(session_id);



    HttpResponse::Ok().json(
        ApiResponse::<()>::new(true, "User Logout Successful".to_string(), None, None)
    )
}



pub fn init_routes(cfg: &mut ServiceConfig) {
    cfg
        .service(register_user)
        .service(login)
        .service(get_profile)
        .service(logout);
}