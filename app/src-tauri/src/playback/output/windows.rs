//! WASAPI2 supplies the device I/O and sample conversion. This bin prevents its
//! implicit shared fallback from reaching PLAYING and keeps gain in software.
use crate::model::AppResult;
use gstreamer::{self as gst, glib, prelude::*, subclass::prelude::*};
use std::sync::OnceLock;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ExclusiveOutput {
        pub endpoint: OnceLock<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExclusiveOutput {
        const NAME: &'static str = "NonsExclusiveOutput";
        type Type = super::ExclusiveOutput;
        type ParentType = gst::Bin;
    }

    impl ObjectImpl for ExclusiveOutput {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecBoolean::builder("exclusive")
                        .read_only()
                        .build(),
                    glib::ParamSpecString::builder("device").read_only().build(),
                ]
            })
        }

        fn property(&self, _: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "exclusive" => true.to_value(),
                "device" => self.endpoint.get().cloned().flatten().to_value(),
                _ => unreachable!(),
            }
        }
    }

    impl GstObjectImpl for ExclusiveOutput {}
    impl ElementImpl for ExclusiveOutput {
        fn change_state(
            &self,
            transition: gst::StateChange,
        ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
            if transition == gst::StateChange::PausedToPlaying {
                if let Err(error) = verify_exclusive(self.endpoint.get().and_then(|e| e.as_deref()))
                {
                    gst::element_error!(self.obj(), gst::ResourceError::OpenWrite, ("{error}"));
                    return Err(gst::StateChangeError);
                }
            }
            self.parent_change_state(transition)
        }
    }
    impl BinImpl for ExclusiveOutput {}
}

glib::wrapper! {
    pub struct ExclusiveOutput(ObjectSubclass<imp::ExclusiveOutput>)
        @extends gst::Bin, gst::Element, gst::Object;
}

fn device_configuration(device: Option<&gst::Device>) -> AppResult<(String, gst::Caps)> {
    if let Some(properties) = device.and_then(|d| d.properties()) {
        if let (Ok(endpoint), Ok(caps)) = (
            properties.get::<String>("device.id"),
            properties.get::<gst::Caps>("device.exclusive-caps"),
        ) {
            if !caps.is_empty() {
                return Ok((endpoint, fixed_device_format(device.unwrap(), caps)));
            }
        }
        if properties.get::<String>("device.api").as_deref() != Ok("wasapi2") {
            return Err("该输出设备不支持 WASAPI2 独占模式".into());
        }
    }
    // The default-device alias has no exclusive caps. Resolve it to a physical
    // endpoint once, so negotiation and the exclusivity check use the same device.
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    monitor.start().map_err(|e| e.to_string())?;
    let devices = monitor.devices();
    monitor.stop();
    let selected = device
        .cloned()
        .or_else(|| {
            devices
                .iter()
                .find(|d| {
                    d.properties()
                        .is_some_and(|p| p.get::<bool>("device.default").unwrap_or(false))
                })
                .cloned()
        })
        .ok_or("没有可用的默认输出设备")?;
    let endpoint = selected
        .properties()
        .and_then(|p| p.get::<String>("device.actual-id").ok())
        .ok_or("无法解析独占输出设备端点")?;
    let physical = devices
        .iter()
        .find(|d| {
            d.properties()
                .is_some_and(|p| p.get::<String>("device.id").ok().as_ref() == Some(&endpoint))
        })
        .ok_or("独占输出设备已断开")?;
    let caps = physical
        .properties()
        .and_then(|p| p.get::<gst::Caps>("device.exclusive-caps").ok())
        .filter(|caps| !caps.is_empty())
        .ok_or("设备未提供可用的独占音频格式，请切回共享模式")?;
    Ok((endpoint, fixed_device_format(physical, caps)))
}

fn fixed_device_format(device: &gst::Device, supported: gst::Caps) -> gst::Caps {
    // Keep a single native format for the lifetime of this output. Otherwise a
    // gapless rate change could reopen WASAPI while PLAYING and silently fall
    // back to shared after the initial exclusivity check. Resample before the sink.
    let preferred = device.caps().and_then(|caps| {
        let structure = caps.structure(0)?;
        Some(
            gst::Caps::builder("audio/x-raw")
                .field("rate", structure.get::<i32>("rate").ok()?)
                .field("channels", structure.get::<i32>("channels").ok()?)
                .build(),
        )
    });
    let caps = preferred
        .map(|preferred| supported.intersect(&preferred))
        .filter(|caps| !caps.is_empty())
        .unwrap_or(supported);
    for format in ["S32LE", "S24_32LE", "S24LE", "F32LE", "S16LE"] {
        let preferred = gst::Caps::builder("audio/x-raw")
            .field("format", format)
            .build();
        let mut candidate = caps.intersect(&preferred);
        if !candidate.is_empty() {
            candidate.fixate();
            return candidate;
        }
    }
    let mut caps = caps;
    caps.fixate();
    caps
}

pub(super) fn create_exclusive_sink(device: Option<&gst::Device>) -> AppResult<gst::Element> {
    let (endpoint, caps) = device_configuration(device)?;
    let bin: ExclusiveOutput = glib::Object::builder()
        .property("name", "nons-output")
        .build();
    let sink = gst::ElementFactory::make("wasapi2sink")
        .name("nons-exclusive-device")
        .property("device", &endpoint)
        .property("exclusive", true)
        .build()
        .map_err(|e| e.to_string())?;
    let volume = gst::ElementFactory::make("volume")
        .name("nons-exclusive-volume")
        .build()
        .map_err(|e| e.to_string())?;
    let convert = gst::ElementFactory::make("audioconvert")
        .build()
        .map_err(|e| e.to_string())?;
    let resample = gst::ElementFactory::make("audioresample")
        .build()
        .map_err(|e| e.to_string())?;
    let filter = gst::ElementFactory::make("capsfilter")
        .name("nons-exclusive-format")
        .property("caps", caps)
        .build()
        .map_err(|e| e.to_string())?;
    // GstBin searches newest children first for GstStreamVolume. Keep software
    // volume first in that search: WASAPI session gain is unavailable in exclusive.
    bin.add_many([&sink, &filter, &resample, &convert, &volume])
        .map_err(|e| e.to_string())?;
    gst::Element::link_many([&volume, &convert, &resample, &filter, &sink])
        .map_err(|e| e.to_string())?;
    let pad =
        gst::GhostPad::builder_with_target(&volume.static_pad("sink").ok_or("独占输出入口不可用")?)
            .map_err(|e| e.to_string())?
            .build();
    bin.add_pad(&pad).map_err(|e| e.to_string())?;
    bin.imp()
        .endpoint
        .set(Some(endpoint))
        .map_err(|_| "独占设备重复设置")?;
    Ok(bin.upcast())
}

/// A shared client must be rejected while this endpoint is held exclusively.
/// No audio is submitted or started by the probe. Any inconclusive result fails
/// closed; the user can explicitly select shared mode instead.
fn verify_exclusive(endpoint: Option<&str>) -> AppResult<()> {
    use ::windows::{
        core::HSTRING,
        Win32::{
            Foundation::RPC_E_CHANGED_MODE,
            Media::Audio::{
                eConsole, eRender, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
                AUDCLNT_E_DEVICE_IN_USE, AUDCLNT_SHAREMODE_SHARED,
            },
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
        },
    };
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }
    // All interfaces and format memory are confined to this thread and released
    // before balancing COM initialization. The format is allocated by WASAPI.
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED);
        if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
            return Err(format!("无法检查独占输出：{initialized:?}"));
        }
        let _apartment = Apartment(initialized.is_ok());
        let check = || -> ::windows::core::Result<()> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = match endpoint {
                Some(id) => enumerator.GetDevice(&HSTRING::from(id))?,
                None => enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?,
            };
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let format = client.GetMixFormat()?;
            let result = client.Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 0, 0, format, None);
            CoTaskMemFree(Some(format.cast()));
            result
        };
        match check() {
            Err(error) if error.code() == AUDCLNT_E_DEVICE_IN_USE => Ok(()),
            Ok(()) => Err("设备未成功进入独占模式，请关闭占用设备的程序或切回共享模式".into()),
            Err(error) => Err(format!("无法确认设备独占状态：{error}")),
        }
    }
}
