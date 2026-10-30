use super::super::*;

const ID: &str = "probe-page";
const PAGE: &str = r#"import{createElement as h,useState}from'./_host/react.mjs';import{usePluginRoute,usePluginNavigate}from'./_host/sdk.mjs';function ProbePage(){const{pathname}=usePluginRoute();const navigate=usePluginNavigate();const[count,setCount]=useState(0);return h('div',{'data-probe-page':true},h('span',{'data-probe-state':true},`${count}:${pathname}`),h('button',{'data-probe-increment':true,onClick:()=>setCount(v=>v+1)},'Increment'),h('button',{'data-probe-child':true,onClick:()=>navigate('/child')},'Child'));}
const generation=new URL(import.meta.url).pathname.split('/').at(-2);
export function Page(){return h('div',{'data-upgrade-generation':generation},h(ProbePage));}
"#;

struct DomReports(tokio::sync::watch::Sender<Option<(String, bool)>>);

pub(super) fn initialize(app: &tauri::AppHandle) {
    app.manage(DomReports(tokio::sync::watch::channel(None).0));
}

pub(super) fn report(app: &tauri::AppHandle, phase: &str, success: bool) -> bool {
    if !phase.starts_with("upgrade-dom-") {
        return false;
    }
    app.state::<DomReports>()
        .0
        .send_replace(Some((phase.into(), success)));
    true
}

fn ensure(condition: bool, message: &str) -> AppResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn prepare(version: &str, variant: &str, activation: &str) -> AppResult<PathBuf> {
    let source = super::directory().join(format!("upgrade-{version}-{variant}"));
    std::fs::create_dir_all(&source).map_err(|error| error.to_string())?;
    std::fs::write(
        source.join("manifest.json"),
        serde_json::json!({
            "id": ID, "name": "Probe Page", "version": version, "frontend": "ui.mjs",
            "permissions": ["ui", "storage"],
            "engines": {"app":"^1.0.0", "pluginApi":"^2.0.0", "uiApi":"^2.0.0"},
            "contributes": {
                "pages":[{"id":"root", "path":"/", "export":"Page"}],
                "navigation":[{"id":"entry", "label":"Probe Page", "page":"root"}]
            }
        })
        .to_string(),
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(source.join("ui.mjs"), format!("{PAGE}\n{activation}"))
        .map_err(|error| error.to_string())?;
    Ok(source)
}

fn ready(manager: &PluginManager, version: &str) -> AppResult<Descriptor> {
    let records = manager.lock()?;
    let record = records.get(ID).ok_or("upgrade probe plugin missing")?;
    ensure(
        record.descriptor.manifest.version == version,
        "upgrade probe wrong version",
    )?;
    ensure(
        record.descriptor.enabled && record.descriptor.loaded && record.ready,
        "upgrade probe not enabled/loaded/ready",
    )?;
    ensure(
        record.descriptor.error.is_none(),
        "upgrade probe retained runtime error",
    )?;
    Ok(record.descriptor.clone())
}

async fn wait_ready(manager: &PluginManager, version: &str) -> AppResult<Descriptor> {
    let mut lifecycle = manager.lifecycle.subscribe();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(descriptor) = ready(manager, version) {
                return Ok(descriptor);
            }
            lifecycle
                .changed()
                .await
                .map_err(|error| error.to_string())?;
        }
    })
    .await
    .map_err(|_| "upgrade probe initial frontend ready timeout".to_string())?
}

async fn visible(manager: &PluginManager, generation: u64) -> AppResult<()> {
    let phase = format!("upgrade-dom-{generation}");
    let mut reports = manager.app.state::<DomReports>().0.subscribe();
    let script = format!(
        r#"(()=>{{const phase={phase};const started=Date.now();let opened=false;const poll=()=>{{const root=document.querySelector('[data-upgrade-generation="{generation}"]');const state=root?.querySelector('[data-probe-state]');if(!root&&!opened){{const nav=[...document.querySelectorAll('button')].find(b=>b.textContent==='Probe Page');if(nav){{opened=true;nav.click();}}}}if(state?.textContent==='0:/')root.querySelector('[data-probe-child]').click();else if(state?.textContent==='0:/child')root.querySelector('[data-probe-increment]').click();if(state?.textContent==='1:/child'){{window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{{phase,success:true}});return;}}if(Date.now()-started>5000){{console.error('upgrade DOM timeout',phase,state?.textContent);window.__TAURI_INTERNALS__.invoke('plugin_probe_report',{{phase,success:false}});return;}}setTimeout(poll,40);}};poll();}})();"#,
        phase = serde_json::to_string(&phase).map_err(|error| error.to_string())?
    );
    manager
        .app
        .get_webview_window("main")
        .ok_or("upgrade probe missing WebView")?
        .eval(&script)
        .map_err(|error| error.to_string())?;
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            if let Some((reported, success)) = reports.borrow_and_update().clone() {
                if reported == phase {
                    return ensure(success, "upgrade probe restored page DOM failed");
                }
            }
            reports.changed().await.map_err(|error| error.to_string())?;
        }
    })
    .await
    .map_err(|_| "upgrade probe missing DOM report".to_string())?
}

pub(super) async fn standalone(manager: &Arc<PluginManager>) -> AppResult<()> {
    let source = prepare(
        "1.0.0",
        "initial",
        "export function activate(client){client.openPage('/child');}",
    )?;
    manager.install(source).await?;
    manager.action(ID, "enable", true).await?;
    let initial = wait_ready(manager, "1.0.0").await?;
    visible(manager, initial.generation).await?;
    manager.database.set(ID, "sample", "123")?;
    run(manager).await
}

pub(super) async fn run(manager: &Arc<PluginManager>) -> AppResult<()> {
    let started = Instant::now();
    let original = wait_ready(manager, "1.0.0").await?;
    let old_scope = manager.scoped(ID, original.generation)?;
    let source = prepare(
        "2.0.0",
        "working",
        r#"export async function activate(client){await client.call('storage.set',{key:'upgrade-activation',value:generation});}"#,
    )?;
    manager.install_reviewed(source, true).await?;
    let upgraded = ready(manager, "2.0.0")?;
    ensure(
        upgraded.generation != original.generation,
        "upgrade did not replace generation",
    )?;
    ensure(
        manager.scoped(ID, original.generation).is_err(),
        "old generation still accepted",
    )?;
    ensure(
        old_scope
            .call("storage.get", r#"{"key":"sample"}"#)
            .await
            .is_err(),
        "old scope still accepted",
    )?;
    check_preserved(manager, &upgraded)?;
    visible(manager, upgraded.generation).await?;
    println!("PLUGIN_UPGRADE_PROBE 2.0.0 real WebView activate/ready, new generation, stale scope, storage and DOM: passed");

    for (variant, activation, expected_error) in [
        (
            "import-error",
            "throw new Error('upgrade-probe-import-error');",
            "upgrade-probe-import-error",
        ),
        (
            "activate-error",
            "export function activate(){throw new Error('upgrade-probe-activate-error');}",
            "upgrade-probe-activate-error",
        ),
        (
            "activate-timeout",
            "export async function activate(){await new Promise(()=>{});}",
            "超时",
        ),
    ] {
        let before = ready(manager, "2.0.0")?;
        let scope = manager.scoped(ID, before.generation)?;
        let authorization = manager.database.authorization_state(ID)?;
        let source = prepare("3.0.0", variant, activation)?;
        let attempt = Instant::now();
        let error = manager
            .install_reviewed(source, true)
            .await
            .err()
            .ok_or_else(|| format!("{variant}: broken frontend installation succeeded"))?;
        ensure(
            error.contains(expected_error),
            &format!("{variant}: unexpected install error: {error}"),
        )?;
        ensure(
            !error.contains("恢复失败"),
            &format!("{variant}: rollback failed: {error}"),
        )?;
        let recovered = ready(manager, "2.0.0")?;
        ensure(
            recovered.generation > before.generation,
            "rollback reused generation",
        )?;
        ensure(
            scope
                .call("storage.get", r#"{"key":"sample"}"#)
                .await
                .is_err(),
            "pre-rollback scope still active",
        )?;
        ensure(
            manager.database.authorization_state(ID)? == authorization,
            "rollback changed authorization",
        )?;
        ensure(
            Manifest::read(&manager.root.join(ID))?.version == "2.0.0",
            "rollback did not restore package on disk",
        )?;
        check_preserved(manager, &recovered)?;
        visible(manager, recovered.generation).await?;
        println!("PLUGIN_UPGRADE_PROBE {variant} -> install Err -> 2.0.0 package/auth/storage/new-generation ready/DOM restored: passed ({} ms)", attempt.elapsed().as_millis());
    }
    println!(
        "PLUGIN_UPGRADE_PROBE local upgrade and frontend rollback suite: passed ({} ms)",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn check_preserved(manager: &PluginManager, descriptor: &Descriptor) -> AppResult<()> {
    ensure(
        manager.database.get(ID, "sample")?.as_deref() == Some("123"),
        "upgrade lost storage",
    )?;
    let activation = serde_json::to_string(&descriptor.generation.to_string())
        .map_err(|error| error.to_string())?;
    ensure(
        manager.database.get(ID, "upgrade-activation")?.as_deref() == Some(activation.as_str()),
        "real frontend activate did not run for current generation",
    )
}
