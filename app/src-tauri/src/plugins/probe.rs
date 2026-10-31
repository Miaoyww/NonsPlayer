//! Opt-in real WebView probe, isolated from the user's installed plugins and queue.
use super::*;
use std::sync::atomic::AtomicBool;
#[path = "upgrade_probe.rs"]
mod upgrade_probe;

pub struct Probe {
    pub success: AtomicBool,
}
pub fn directory() -> PathBuf {
    std::env::temp_dir().join(format!("nons-plugin-probe-{}", std::process::id()))
}
pub fn start(manager: Arc<PluginManager>) {
    upgrade_probe::initialize(&manager.app);
    manager.app.manage(Probe {
        success: AtomicBool::new(false),
    });
    let watchdog = manager.app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        eprintln!("PLUGIN_PROBE timeout");
        watchdog.exit(1);
    });
    tauri::async_runtime::spawn(async move {
        if std::env::var_os("NONS_PLUGIN_UPGRADE_PROBE").is_some() {
            match upgrade_probe::standalone(&manager).await {
                Ok(()) => {
                    manager
                        .app
                        .state::<Probe>()
                        .success
                        .store(true, Ordering::SeqCst);
                    println!("PLUGIN_UPGRADE_PROBE passed");
                    manager.app.exit(0);
                }
                Err(error) => {
                    eprintln!("PLUGIN_UPGRADE_PROBE failed: {error}");
                    manager.app.exit(1);
                }
            }
            return;
        }
        let result = plugin_probe_report(
            "page-start".into(),
            true,
            manager.app.clone(),
            manager.app.state(),
        )
        .await;
        if let Err(error) = result {
            eprintln!("PLUGIN_PROBE failed: {error}");
            manager.app.exit(1);
        }
    });
}
#[tauri::command]
pub async fn plugin_probe_report(
    phase: String,
    success: bool,
    app: tauri::AppHandle,
    manager: State<'_, Arc<PluginManager>>,
) -> AppResult<()> {
    if upgrade_probe::report(&app, &phase, success) {
        return Ok(());
    }
    if !success {
        eprintln!("PLUGIN_PROBE {phase} failed");
        app.exit(1);
        return Ok(());
    }
    if phase == "page-start" {
        let snapshot = manager.player.snapshot()?;
        assert!(snapshot.queue.is_empty());
        let source = directory().join("probe-source");
        std::fs::create_dir_all(&source).map_err(|e| e.to_string())?;
        std::fs::write(source.join("manifest.json"), serde_json::json!({"id":"probe-page","name":"Probe Page","version":"1.0.0","frontend":"ui.mjs","permissions":["ui","storage"],"engines":{"app":"^1.0.0","pluginApi":"^2.0.0","uiApi":"^2.0.0"},"contributes":{"pages":[{"id":"root","path":"/","export":"Page"}],"navigation":[{"id":"entry","label":"Probe Page","page":"root"}]}}).to_string()).map_err(|e| e.to_string())?;
        std::fs::write(source.join("ui.mjs"), r#"import{createElement as h,useState}from'./_host/react.mjs';import{usePluginRoute,usePluginNavigate}from'./_host/sdk.mjs';export function Page(){const{pathname}=usePluginRoute();const navigate=usePluginNavigate();const[count,setCount]=useState(0);return h('div',{'data-probe-page':true},h('span',{'data-probe-state':true},`${count}:${pathname}`),h('button',{'data-probe-increment':true,onClick:()=>setCount(v=>v+1)},'Increment'),h('button',{'data-probe-child':true,onClick:()=>navigate('/child')},'Child'));}"#).map_err(|e| e.to_string())?;
        manager.inner().install(source).await?;
        manager.inner().action("probe-page", "enable", true).await?;
        let descriptor = manager
            .list()?
            .into_iter()
            .find(|p| p.manifest.id == "probe-page")
            .ok_or("页面插件缺失")?;
        let context = manager.scoped("probe-page", descriptor.generation)?;
        context
            .call("storage.set", r#"{"key":"sample","value":123}"#)
            .await?;
        assert_eq!(
            context.call("storage.get", r#"{"key":"sample"}"#).await?,
            "123"
        );
        app.get_webview_window("main").ok_or("缺少 WebView")?.eval(r#"(()=>{const started=Date.now();let step=0;const poll=()=>{const button=[...document.querySelectorAll('button')].find(b=>b.textContent==='Probe Page');const state=document.querySelector('[data-probe-state]');if(step===0&&button){button.click();step=1;}else if(step===1&&state?.textContent==='0:/'){document.querySelector('[data-probe-increment]').click();step=2;}else if(step===2&&state?.textContent==='1:/'){document.querySelector('[data-probe-child]').click();step=3;}else if(step===3&&state?.textContent==='1:/child'){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'page',success:true});return;}if(Date.now()-started>8000){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'page',success:false});return;}setTimeout(poll,50);};poll();})();"#).map_err(|e| e.to_string())?;
    } else if phase == "page" {
        println!("PLUGIN_PROBE directory install, scoped storage, navigation, page and subroute state: passed");
        if let Err(error) = upgrade_probe::run(manager.inner()).await {
            eprintln!("PLUGIN_UPGRADE_PROBE failed: {error}");
            app.exit(1);
            return Err(error);
        }
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/plugin-fixtures/settings-fixture");
        manager.inner().install(fixture).await?;
        manager
            .inner()
            .action("settings-fixture", "enable", true)
            .await?;
        app.get_webview_window("main").ok_or("缺少 WebView")?.eval(r#"
          (async()=>{
            const invoke=window.__TAURI_INTERNALS__.invoke;
            const wait=async(find)=>{const start=Date.now();while(Date.now()-start<8000){const found=find();if(found)return found;await new Promise(r=>setTimeout(r,40));}throw Error('DOM timeout');};
            const waitAsync=async(find)=>{const start=Date.now();while(Date.now()-start<8000){if(await find())return;await new Promise(r=>setTimeout(r,40));}throw Error('save timeout');};
            const button=(text)=>[...document.querySelectorAll('[role=dialog] button')].find(b=>b.textContent.trim()===text);
            try{
              (await wait(()=>document.querySelector('button[aria-label="设置"]'))).click();
              (await wait(()=>button('插件设置'))).click();
              (await wait(()=>document.querySelector('button[aria-label="设置 配置示例"]'))).click();
              const input=await wait(()=>document.querySelector('[aria-label="欢迎语"]'));
              await wait(()=>input.value);
              const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
              input.focus();setter.call(input,'probe-config');input.dispatchEvent(new Event('input',{bubbles:true}));
              await new Promise(r=>setTimeout(r,0));input.blur();
              const fixture=(await invoke('plugin_list')).find(p=>p.manifest.id==='settings-fixture');
              await waitAsync(async()=>(await invoke('plugin_settings',{id:fixture.manifest.id,generation:fixture.generation,operation:'get',args:{}})).snapshot.values.greeting==='probe-config');
              (await wait(()=>button('打开子页面'))).click();
              await wait(()=>document.querySelector('[data-settings-route]')?.textContent==='/settings/advanced');
              if(document.querySelector('[data-probe-state]')?.textContent!=='1:/child')throw Error('main route changed');
              button('写入并读取专属文件').click();
              await wait(()=>[...document.querySelectorAll('[role=dialog] [role=status]')].some(p=>p.textContent==='文件内容：probe-config'));
              document.querySelector('[aria-label="关闭设置"]').click();
              await invoke('plugin_probe_report',{phase:'configuration',success:true});
            }catch(error){console.error(error);await invoke('plugin_probe_report',{phase:'configuration',success:false});}
          })();
        "#).map_err(|e| e.to_string())?;
    } else if phase == "configuration" {
        let configs = manager.configurations.snapshot("settings-fixture")?;
        assert_eq!(configs.values["greeting"], "probe-config");
        let data = manager.files.data_directory("settings-fixture")?;
        assert_eq!(
            std::fs::read(data.join("sample.bin")).map_err(|e| e.to_string())?,
            b"probe-config"
        );
        let external = directory().join("external");
        std::fs::create_dir_all(&external).map_err(|e| e.to_string())?;
        std::fs::write(external.join("keep"), b"external").map_err(|e| e.to_string())?;
        manager
            .files
            .grant("settings-fixture", external.clone(), true)?;
        manager
            .inner()
            .action("settings-fixture", "uninstall-keep-data", false)
            .await?;
        assert!(manager.files.grants("settings-fixture")?.is_empty());
        assert!(data.join("sample.bin").exists());
        assert_eq!(
            manager.database.config_read("settings-fixture")?.1["greeting"],
            "probe-config"
        );
        manager
            .inner()
            .install(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("target/plugin-fixtures/settings-fixture"),
            )
            .await?;
        assert_eq!(
            manager.configurations.snapshot("settings-fixture")?.values["greeting"],
            "probe-config"
        );
        manager
            .inner()
            .action("settings-fixture", "uninstall", false)
            .await?;
        assert!(!data.exists());
        assert_eq!(manager.database.config_read("settings-fixture")?.0, 0);
        assert!(external.join("keep").exists());
        println!("PLUGIN_PROBE custom configuration routes, binary SDK, retained reinstall and uninstall cleanup: passed");
        manager
            .inner()
            .action("probe-page", "uninstall", false)
            .await?;
        assert!(manager.database.get("probe-page", "sample")?.is_none());
        app.get_webview_window("main").ok_or("缺少 WebView")?.eval(r#"(()=>{const start=Date.now();const poll=()=>{const nav=[...document.querySelectorAll('button')].some(b=>b.textContent==='Probe Page');if(!document.querySelector('[data-probe-page]')&&!nav){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'removed',success:true});return;}if(Date.now()-start>3000){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'removed',success:false});return;}setTimeout(poll,50);};poll();})();"#).map_err(|e| e.to_string())?;
    } else if phase == "removed" {
        println!(
            "PLUGIN_PROBE uninstall removes storage, page and navigation contributions: passed"
        );
        let source = directory().join("probe-window-source");
        std::fs::create_dir_all(&source).map_err(|error| error.to_string())?;
        std::fs::write(
            source.join("manifest.json"),
            serde_json::json!({
                "id":"probe-window", "name":"Probe Window", "version":"1.0.0", "frontend":"ui.mjs",
                "permissions":["ui","window:manage","player:read"],
                "engines":{"app":"^1.0.0","pluginApi":"^2.0.0","uiApi":"^2.0.0"},
                "contributes":{"windows":[{"id":"lyrics","export":"Lyrics"}]}
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
        std::fs::write(source.join("ui.mjs"), r#"import{createElement as h,useState}from'./_host/react.mjs';import{usePlayer}from'./_host/sdk.mjs';export function Lyrics(){const state=usePlayer();const[count,setCount]=useState(0);return h('div',{'data-probe-window':true},h('span',{'data-probe-volume':true},String(state.volume)),h('button',{'data-probe-tick':true,onClick:()=>setCount(v=>v+1)},String(count)));}"#).map_err(|error| error.to_string())?;
        manager.inner().install(source).await?;
        manager
            .inner()
            .action("probe-window", "enable", true)
            .await?;
        let descriptor = manager
            .list()?
            .into_iter()
            .find(|plugin| plugin.manifest.id == "probe-window")
            .ok_or("窗口插件缺失")?;
        let context = manager.scoped("probe-window", descriptor.generation)?;
        let mut denied = context.clone();
        denied.permissions.remove("window:manage");
        assert!(denied
            .call("windows.open", r#"{"id":"lyrics"}"#)
            .await
            .is_err());
        assert!(context
            .call("windows.open", r#"{"id":"undeclared"}"#)
            .await
            .is_err());
        let value: serde_json::Value = serde_json::from_str(&context.call("windows.open", r#"{"id":"lyrics","width":400,"height":100,"transparent":true,"decorations":false,"alwaysOnTop":true}"#).await?).map_err(|error| error.to_string())?;
        let window = app
            .get_webview_window(value["label"].as_str().ok_or("窗口标签缺失")?)
            .ok_or("插件窗口未创建")?;
        assert!(window
            .is_always_on_top()
            .map_err(|error| error.to_string())?);
        context.call("windows.update", r#"{"id":"lyrics","width":420,"height":120,"x":20,"y":20,"ignoreCursorEvents":true}"#).await?;
        context
            .call(
                "windows.update",
                r#"{"id":"lyrics","ignoreCursorEvents":false}"#,
            )
            .await?;
        context.call("windows.hide", r#"{"id":"lyrics"}"#).await?;
        assert!(!window.is_visible().map_err(|error| error.to_string())?);
        context.call("windows.show", r#"{"id":"lyrics"}"#).await?;
        let mut other = context.clone();
        other.id = "other-plugin".into();
        assert!(other
            .call("windows.close", r#"{"id":"lyrics"}"#)
            .await
            .is_err());
        window.eval(r#"(()=>{const start=Date.now();const poll=()=>{if(document.querySelector('[data-probe-window]')){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'window-rendered',success:true});return;}if(Date.now()-start>8000){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'window-rendered',success:false});return;}setTimeout(poll,40);};poll();})();"#).map_err(|error| error.to_string())?;
    } else if phase == "window-rendered" {
        app.get_window("main")
            .ok_or("主窗口缺失")?
            .hide()
            .map_err(|error| error.to_string())?;
        app.get_webview("main")
            .ok_or("主 WebView 缺失")?
            .close()
            .map_err(|error| error.to_string())?;
        manager.inner().frontend_detached()?;
        let deadline = Instant::now();
        while app.get_webview("main").is_some() {
            if deadline.elapsed() > Duration::from_secs(3) {
                return Err("主 WebView 未释放".into());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        manager.player.send(crate::player::Command::Volume(0.42))?;
        let window = app
            .webview_windows()
            .into_iter()
            .find(|(label, _)| label.starts_with("plugin-window-probe-window-"))
            .map(|(_, window)| window)
            .ok_or("独立窗口缺失")?;
        window.eval(r#"(()=>{const start=Date.now();let clicked=false;const poll=()=>{const button=document.querySelector('[data-probe-tick]');if(button&&!clicked){clicked=true;button.click();}if(document.querySelector('[data-probe-volume]')?.textContent==='0.42'&&button?.textContent==='1'){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'window-independent',success:true});return;}if(Date.now()-start>5000){window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{phase:'window-independent',success:false});return;}setTimeout(poll,40);};poll();})();"#).map_err(|error| error.to_string())?;
    } else if phase == "window-independent" {
        assert!(app.get_webview("main").is_none());
        println!("PLUGIN_PROBE independent window SDK playback subscription and React updates after main WebView release: passed");
        let descriptor = manager
            .list()?
            .into_iter()
            .find(|plugin| plugin.manifest.id == "probe-window")
            .ok_or("窗口插件缺失")?;
        let context = manager.scoped("probe-window", descriptor.generation)?;
        let prefix = format!(
            "plugin-window-probe-window-{}-lyrics-",
            descriptor.generation
        );
        let label = app
            .webview_windows()
            .keys()
            .find(|label| label.starts_with(&prefix))
            .cloned()
            .ok_or("插件窗口未找到")?;
        context.call("windows.close", r#"{"id":"lyrics"}"#).await?;
        let deadline = Instant::now();
        while app.get_webview_window(&label).is_some() {
            if deadline.elapsed() > Duration::from_secs(3) {
                return Err("插件窗口未关闭".into());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let opened: serde_json::Value =
            serde_json::from_str(&context.call("windows.open", r#"{"id":"lyrics"}"#).await?)
                .map_err(|error| error.to_string())?;
        let label = opened["label"].as_str().ok_or("窗口标签缺失")?;
        manager
            .inner()
            .action("probe-window", "disable", false)
            .await?;
        let deadline = Instant::now();
        while app.get_webview_window(label).is_some() {
            if deadline.elapsed() > Duration::from_secs(3) {
                return Err("停用后插件窗口未关闭".into());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(context
            .call("windows.open", r#"{"id":"lyrics"}"#)
            .await
            .is_err());
        manager
            .inner()
            .action("probe-window", "uninstall", false)
            .await?;
        println!("PLUGIN_PROBE independent plugin window rendering, geometry, visibility, permissions and disable cleanup: passed");
        app.state::<Probe>().success.store(true, Ordering::SeqCst);
        app.exit(0);
    }
    Ok(())
}
