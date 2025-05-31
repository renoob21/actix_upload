use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::user::UserData;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    message: String,
    data: Option<T>,
    error: Option<String>
}

impl<T> ApiResponse<T> {
    pub fn new(success: bool, message: String, data: Option<T>, error: Option<String>) -> Self {
        ApiResponse { success, message, data, error}
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    session_id: Uuid,
    pub user_data: UserData,
    pub last_active: DateTime<Utc>,
}

impl Session {
    pub fn new(session_id: Uuid, user_data: UserData) -> Self {
        let last_active = Utc::now();

        Session { session_id, user_data, last_active }
    }
}