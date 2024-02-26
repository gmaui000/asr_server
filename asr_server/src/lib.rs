pub mod base;
pub mod error;
pub mod asr;

use base::record::QueryTracker;

// 定义全局状态
pub struct AppState {
    pub track: QueryTracker,
}

