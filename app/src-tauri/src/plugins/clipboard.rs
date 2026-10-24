use super::PluginManager;
use crate::model::AppResult;
use clipboard_rs::{Clipboard, ClipboardContext, ClipboardHandler};
#[cfg(not(windows))]
use clipboard_rs::{ClipboardWatcher, ClipboardWatcherContext};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};
pub enum Shutdown {
    #[cfg(windows)]
    Windows(clipboard_win::monitor::Shutdown),
    #[cfg(not(windows))]
    Other(clipboard_rs::WatcherShutdown),
}
impl Shutdown {
    pub fn stop(self) {
        match self {
            #[cfg(windows)]
            Self::Windows(channel) => drop(channel),
            #[cfg(not(windows))]
            Self::Other(channel) => channel.stop(),
        }
    }
}

struct Handler {
    context: ClipboardContext,
    manager: Weak<PluginManager>,
}
impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) {
        let Some(manager) = self.manager.upgrade() else {
            return;
        };
        if !manager.has_clipboard_subscribers() {
            return;
        }
        let Ok(text) = read_context(&self.context) else {
            return;
        };
        // Coalesce changes; the host does not interpret clipboard contents.
        manager.queue_clipboard(text);
    }
}
pub const MAX_TEXT: usize = 16 * 1024;

fn read_context(context: &ClipboardContext) -> AppResult<String> {
    let text = context
        .get_text()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("plugins.clipboard.read_text", error);
        })
        .map_err(|_| "剪贴板中没有可读取的文本")?;
    if text.len() > MAX_TEXT {
        return Err("剪贴板文本超过 16KiB".into());
    }
    Ok(text)
}

pub fn read_text() -> AppResult<String> {
    let context = ClipboardContext::new().map_err(|e| e.to_string())?;
    read_context(&context)
}
#[cfg(windows)]
pub fn start(manager: &Arc<PluginManager>) -> AppResult<Shutdown> {
    let manager = Arc::downgrade(manager);
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("nons-plugin-clipboard".into())
        .spawn(move || {
            let result = (|| {
                let monitor = clipboard_win::Monitor::new().map_err(|e| e.to_string())?;
                let context = ClipboardContext::new().map_err(|e| e.to_string())?;
                Ok::<_, String>((monitor, Handler { context, manager }))
            })();
            match result {
                Ok((mut monitor, mut handler)) => {
                    if sender
                        .send(Ok(Shutdown::Windows(monitor.shutdown_channel())))
                        .is_err()
                    {
                        return;
                    }
                    while let Ok(true) = monitor.recv() {
                        handler.on_clipboard_change();
                    }
                }
                Err(error) => {
                    let _ = (sender.send(Err(error))).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("sender.send", error);
                    });
                }
            }
        })
        .map_err(|e| e.to_string())?;
    receiver
        .recv_timeout(Duration::from_secs(2))
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn(
                "plugins.clipboard.watch.initialize_timeout",
                error,
            );
        })
        .map_err(|_| "剪贴板监听初始化超时")?
}
#[cfg(not(windows))]
pub fn start(manager: &Arc<PluginManager>) -> AppResult<Shutdown> {
    let context = ClipboardContext::new().map_err(|e| format!("剪贴板不可用: {e}"))?;
    let mut watcher = ClipboardWatcherContext::new_with_interval(Duration::from_millis(500))
        .map_err(|e| e.to_string())?;
    let shutdown = watcher
        .add_handler(Handler {
            context,
            manager: Arc::downgrade(manager),
        })
        .get_shutdown_channel();
    std::thread::Builder::new()
        .name("nons-plugin-clipboard".into())
        .spawn(move || watcher.start_watch())
        .map_err(|e| e.to_string())?;
    Ok(Shutdown::Other(shutdown))
}
