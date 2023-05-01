#[derive(Debug)]
pub struct CustomError {
    message: String,
}

impl CustomError {
    pub fn new(message: impl Into<String>) -> Self {
        CustomError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl std::error::Error for CustomError {}
