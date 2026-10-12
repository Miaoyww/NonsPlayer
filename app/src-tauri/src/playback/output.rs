//! Owns platform output selection; exclusive requests must never silently become shared.
use crate::model::{AppResult, AudioOutputMode};
use gstreamer::{self as gst, prelude::*};

#[cfg(windows)]
#[path = "output/windows.rs"]
mod windows;

pub fn available_modes() -> Vec<AudioOutputMode> {
    if cfg!(windows) {
        vec![AudioOutputMode::Shared, AudioOutputMode::Exclusive]
    } else {
        Vec::new()
    }
}

pub fn create_sink(device: Option<&gst::Device>, mode: AudioOutputMode) -> AppResult<gst::Element> {
    if mode == AudioOutputMode::Exclusive {
        #[cfg(windows)]
        return windows::create_exclusive_sink(device);
        #[cfg(not(windows))]
        return Err("WASAPI 独占模式仅支持 Windows".into());
    }
    let selected = device
        .map(|device| device.create_element(Some("nons-output")))
        .transpose()
        .map_err(|e| e.to_string())?;
    let sink = match selected {
        Some(sink) => sink,
        None => gst::ElementFactory::make(if cfg!(windows) {
            "wasapi2sink"
        } else {
            "autoaudiosink"
        })
        .name("nons-output")
        .build()
        .map_err(|e| e.to_string())?,
    };
    if sink.find_property("exclusive").is_some() {
        sink.set_property("exclusive", false);
    }
    Ok(sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_saved_output_defaults_to_shared_and_modes_round_trip() {
        let state = crate::model::PlayerSnapshot::default();
        let mut saved = serde_json::to_value(state).unwrap();
        saved.as_object_mut().unwrap().remove("audioOutputMode");
        let restored: crate::model::PlayerSnapshot = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.audio_output_mode, AudioOutputMode::Shared);
        for mode in [AudioOutputMode::Shared, AudioOutputMode::Exclusive] {
            assert_eq!(
                serde_json::from_str::<AudioOutputMode>(&serde_json::to_string(&mode).unwrap())
                    .unwrap(),
                mode
            );
        }
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "requires a Windows device advertising exclusive output formats"]
    fn windows_modes_use_explicit_sinks_without_shared_fallback() {
        gst::init().unwrap();
        let shared = create_sink(None, AudioOutputMode::Shared).unwrap();
        assert_eq!(shared.factory().unwrap().name(), "wasapi2sink");
        assert!(!shared.property::<bool>("exclusive"));
        let exclusive = create_sink(None, AudioOutputMode::Exclusive).unwrap();
        let bin = exclusive.downcast_ref::<gst::Bin>().unwrap();
        assert_eq!(
            bin.by_name("nons-exclusive-device")
                .unwrap()
                .factory()
                .unwrap()
                .name(),
            "wasapi2sink"
        );
        assert!(exclusive.property::<bool>("exclusive"));
        assert!(exclusive.find_property("volume").is_none());
    }

    #[test]
    #[cfg(not(windows))]
    fn other_platforms_do_not_advertise_or_accept_wasapi() {
        assert!(available_modes().is_empty());
        assert!(create_sink(None, AudioOutputMode::Exclusive).is_err());
    }
}
