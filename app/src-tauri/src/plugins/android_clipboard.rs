//! Desktop clipboard monitoring is deliberately unavailable in the Android trial.
use super::PluginManager;
use crate::model::AppResult;
use std::sync::Arc;

pub const MAX_TEXT: usize = 16 * 1024;
pub struct Shutdown;
impl Shutdown {
    pub fn stop(self) {}
}
pub fn read_text() -> AppResult<String> {
    Err("Android 试用版暂不支持插件剪贴板访问".into())
}
pub fn start(_manager: &Arc<PluginManager>) -> AppResult<Shutdown> {
    Err("Android 试用版暂不支持插件剪贴板监听".into())
}
