use actix_web::*;
use actix_files as fs;
use std::sync::{Arc, Mutex};
use super::api::{asr_handler, index};
use super::super::base::configuration::AppConfigItem;
use super::super::{ AppState, QueryTracker};
use tracing::{self, info};
use chrono::{Local, Datelike, Timelike};

#[actix_web::main]
pub async fn start(config: &AppConfigItem) -> anyhow::Result<()> {

    let now = Local::now();
    let nowtime = format!("{:02}/{:02}/{:04} {:02}:{:02}:{:02}", now.month(), now.day(), now.year(), now.hour(), now.minute(), now.second());
    info!("tts_server start at {}.", nowtime);



    let app_state = web::Data::new(Arc::new(Mutex::new(AppState {
        track: QueryTracker::new(nowtime),
    })));

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .app_data(web::Data::new(actix_web::web::PayloadConfig::new(10 * 1024 * 1024)))// 设置为 10MB
            .service(asr_handler::api_asr)
            .service(index::index)
            .service(fs::Files::new("/demo", "demo"))
            .configure(init)
    })
    .bind((config.ip.clone(), config.port))?
    .run()
    .await?;
    Ok(())
}

fn init(_cfg: &mut web::ServiceConfig) {

}
