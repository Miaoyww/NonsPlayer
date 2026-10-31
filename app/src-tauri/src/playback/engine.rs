use crate::model::AppResult;
use std::sync::Arc;

pub(crate) type Resource = Option<Arc<crate::music::manager::ResourceTicket>>;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutputDevice {
    pub id: String,
    pub name: String,
}

#[cfg(not(target_os = "android"))]
pub(crate) fn output_devices() -> AppResult<Vec<OutputDevice>> {
    super::native_audio::output_devices()
}

#[cfg(target_os = "android")]
pub(crate) fn output_devices() -> AppResult<Vec<OutputDevice>> {
    Ok(Vec::new())
}

#[cfg(target_os = "android")]
pub(crate) use super::web_audio::{Control, Feedback, FeedbackKind, WebAudio as AudioEngine};

#[cfg(not(target_os = "android"))]
pub(crate) use super::native_audio::{AudioEngine, Control, Feedback, FeedbackKind};

#[cfg(target_os = "android")]
pub(crate) fn create(
    _sender: std::sync::mpsc::SyncSender<super::player::Command>,
) -> AppResult<AudioEngine> {
    AudioEngine::new()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn create(
    _sender: std::sync::mpsc::SyncSender<super::player::Command>,
) -> AppResult<AudioEngine> {
    AudioEngine::new()
}
