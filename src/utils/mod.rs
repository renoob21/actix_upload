use std::{fs, io, path::Path};

use actix_multipart::form::tempfile::TempFile;
use actix_web::{web, HttpRequest};
use chrono::Utc;
use models::{ Session};

use crate::AppState;

pub mod models;

pub async fn save_uploaded_file(temp_file: &TempFile, target_path: &str) -> io::Result<()> {
    let tmp_path = temp_file.file.path();

    let destination = Path::new(target_path);

    if let Some(parent_dir) = destination.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    fs::copy(tmp_path, destination)?;

    Ok(())
}

pub async fn get_session(app_state: web::Data<AppState>, req: &HttpRequest) -> Result<Session, String> {
    let session_id = match req.headers().get("session_id") {
        None => return Err("Required header \'session_id\'".to_string()),
        Some(id) => id.to_str().unwrap()
    };



    let mut session_store = app_state.session_store.lock().unwrap();
    

    let user_session = match session_store.get_mut(session_id) {
        None => return Err("Session Invalid".to_string()),
        Some(session) => session
    };

    let time_from_last_online = Utc::now() - user_session.last_active;


    if time_from_last_online.num_hours() >= 48 {
        session_store.remove(session_id);
        return Err("Session expired".to_string());
    }

    user_session.last_active = Utc::now();

    Ok(user_session.clone())
}