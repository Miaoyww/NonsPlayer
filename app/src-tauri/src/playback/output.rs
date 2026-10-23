//! Owns shared audio output selection.
use crate::model::AppResult;
use gstreamer::{self as gst, prelude::*};

pub fn start_device_monitor() -> AppResult<Option<gst::DeviceMonitor>> {
    // Android's OpenSL ES sink follows system routing and has no device provider.
    // Requiring a desktop device monitor would stop the playback actor at startup.
    #[cfg(target_os = "android")]
    return Ok(None);

    #[cfg(not(target_os = "android"))]
    {
        let monitor = gst::DeviceMonitor::new();
        monitor.add_filter(Some("Audio/Sink"), None);
        monitor
            .start()
            .map_err(|e| format!("无法监视音频设备：{e}"))?;
        Ok(Some(monitor))
    }
}

pub fn create_sink(device: Option<&gst::Device>) -> AppResult<gst::Element> {
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
    #[test]
    fn legacy_exclusive_snapshot_restores_without_output_mode() {
        let mut saved = serde_json::to_value(crate::model::PlayerSnapshot::default()).unwrap();
        saved["audioOutputMode"] = serde_json::json!("exclusive");
        saved["positionMs"] = serde_json::json!(12345);
        let restored: crate::model::PlayerSnapshot = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.position_ms, 12345);
        assert!(serde_json::to_value(restored)
            .unwrap()
            .get("audioOutputMode")
            .is_none());
    }
}
