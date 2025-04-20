use serde::Serialize;

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