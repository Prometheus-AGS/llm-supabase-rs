pub mod api;
pub mod app;
pub mod config;
pub mod features;
pub mod infrastructure;
pub mod models;
pub mod shared;

// Re-export commonly used types
pub use config::AppConfig;
pub use shared::{AppError, AppResult};

// Re-export main application
pub use app::App;
