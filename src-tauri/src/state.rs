use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::services::onnx_service::OnnxService;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub onnx_service: Mutex<Option<OnnxService>>,
    pub app_data_dir: PathBuf,
    pub models_dir: PathBuf,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> anyhow::Result<Self> {
        let models_dir = app_data_dir.join("models");
        std::fs::create_dir_all(&models_dir)?;

        let db_path = app_data_dir.join("figurine_studio.db");
        let conn = Connection::open(&db_path)?;
        crate::db::initialize(&conn)?;

        Ok(Self {
            db: Mutex::new(conn),
            onnx_service: Mutex::new(None),
            app_data_dir,
            models_dir,
        })
    }
}
