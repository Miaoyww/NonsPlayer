//! User-authorized native processes, owned by a functional-plugin generation.
use super::capability::Context;
use crate::model::AppResult;
use serde_json::Value;

#[derive(Default)]
pub struct Processes {
    #[cfg(windows)]
    tasks: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<u64, native::Task>>>,
}

impl Processes {
    pub async fn call(&self, context: &Context, operation: &str, args: &Value) -> AppResult<Value> {
        context.check(Some("process:spawn"))?;
        #[cfg(not(windows))]
        {
            let _ = (operation, args);
            Err("插件子进程目前仅支持 Windows".into())
        }
        #[cfg(windows)]
        {
            let tasks = self.tasks.clone();
            let plugin = context.id.clone();
            let generation = context.generation;
            let active = context.active.clone();
            let operation = operation.to_owned();
            let args = args.clone();
            tauri::async_runtime::spawn_blocking(move || {
                native::call(&tasks, &plugin, generation, &active, &operation, &args)
            })
            .await
            .map_err(|_| "子进程操作线程失败".to_owned())?
        }
    }

    pub fn revoke(&self, plugin: &str, generation: u64) {
        #[cfg(windows)]
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.retain(|_, task| task.plugin != plugin || task.generation != generation);
        }
        #[cfg(not(windows))]
        let _ = (plugin, generation);
    }

    pub fn shutdown(&self) {
        #[cfg(windows)]
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.clear();
        }
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::Deserialize;
    use serde_json::json;
    use std::{
        collections::{BTreeMap, VecDeque},
        fs::File,
        io::{Read, Write},
        mem::{size_of, size_of_val},
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc, Mutex,
        },
    };
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{
                SetHandleInformation, HANDLE, HANDLE_FLAGS, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0,
            },
            Security::SECURITY_ATTRIBUTES,
            System::{
                JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                    SetInformationJobObject, TerminateJobObject,
                    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                },
                Pipes::CreatePipe,
                Threading::{
                    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
                    InitializeProcThreadAttributeList, ResumeThread, TerminateProcess,
                    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_NO_WINDOW,
                    CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST,
                    PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, STARTF_USESTDHANDLES,
                    STARTUPINFOEXW,
                },
            },
        },
    };

    const OUTPUT_BYTES: usize = 256 * 1024;
    const CHUNK_BYTES: usize = 16 * 1024;
    const PER_PLUGIN: usize = 4;
    const GLOBAL: usize = 16;

    #[derive(Default)]
    struct Output {
        bytes: VecDeque<u8>,
        total: u64,
        eof: bool,
        error: Option<String>,
    }
    impl Output {
        fn append(&mut self, bytes: &[u8]) {
            self.total += bytes.len() as u64;
            let overflow = (self.bytes.len() + bytes.len()).saturating_sub(OUTPUT_BYTES);
            self.bytes.drain(..overflow.min(self.bytes.len()));
            self.bytes.extend(
                bytes
                    .iter()
                    .copied()
                    .skip(bytes.len().saturating_sub(OUTPUT_BYTES)),
            );
        }
        fn read(&self, args: &Value) -> AppResult<Value> {
            let first = self.total - self.bytes.len() as u64;
            let requested = args
                .get("cursor")
                .map(|v| v.as_u64().ok_or("输出游标无效"))
                .transpose()?
                .unwrap_or(first);
            if requested > self.total {
                return Err("输出游标超过已产生的数据".into());
            }
            let cursor = requested.max(first);
            let max = args
                .get("maxBytes")
                .map(|v| v.as_u64().ok_or("读取大小无效"))
                .transpose()?
                .unwrap_or(CHUNK_BYTES as u64);
            if max == 0 || max > CHUNK_BYTES as u64 {
                return Err("单次输出读取须为 1 至 16384 字节".into());
            }
            let bytes: Vec<u8> = self
                .bytes
                .iter()
                .skip((cursor - first) as usize)
                .take(max as usize)
                .copied()
                .collect();
            let next = cursor + bytes.len() as u64;
            Ok(
                json!({"data":STANDARD.encode(&bytes),"cursor":cursor,"nextCursor":next,"total":self.total,"dropped":first,"eof":self.eof && next == self.total,"error":self.error}),
            )
        }
    }

    pub(super) struct Task {
        pub plugin: String,
        pub generation: u64,
        job: OwnedHandle,
        process: OwnedHandle,
        pid: u32,
        terminated: bool,
        stdin: Option<mpsc::SyncSender<Vec<u8>>>,
        stdout: Arc<Mutex<Output>>,
        stderr: Arc<Mutex<Output>>,
    }
    impl Task {
        fn terminate(&mut self) -> AppResult<()> {
            unsafe { TerminateJobObject(handle(&self.job), 1) }.map_err(|e| e.to_string())?;
            self.terminated = true;
            self.stdin = None;
            Ok(())
        }
        fn snapshot(&self, id: u64) -> AppResult<Value> {
            let exited = unsafe { WaitForSingleObject(handle(&self.process), 0) } == WAIT_OBJECT_0;
            let mut code = 0;
            if exited {
                unsafe { GetExitCodeProcess(handle(&self.process), &mut code) }
                    .map_err(|e| e.to_string())?;
            }
            Ok(
                json!({"id":id,"pid":self.pid,"state":if self.terminated { "terminated" } else if exited { "exited" } else { "running" },"exitCode":if exited { Some(code) } else { None }}),
            )
        }
    }
    impl Drop for Task {
        fn drop(&mut self) {
            self.stdin = None;
            // Closing the last non-inherited job handle kills every descendant as well.
        }
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Spawn {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        cwd: Option<String>,
    }
    fn validate_spawn(value: &Value) -> AppResult<Spawn> {
        let request: Spawn = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if request.program.is_empty()
            || !Path::new(&request.program).is_absolute()
            || request.program.contains(['\0', '"'])
        {
            return Err("可执行文件须为不含引号的绝对路径".into());
        }
        if request.args.len() > 128
            || request.args.iter().any(|s| s.contains('\0'))
            || request
                .cwd
                .as_ref()
                .is_some_and(|s| s.contains('\0') || !Path::new(s).is_absolute())
        {
            return Err("参数最多 128 项，工作目录须为绝对路径，参数不能含 NUL".into());
        }
        Ok(request)
    }

    pub(super) fn call(
        tasks: &Mutex<BTreeMap<u64, Task>>,
        plugin: &str,
        generation: u64,
        active: &AtomicBool,
        operation: &str,
        args: &Value,
    ) -> AppResult<Value> {
        let mut tasks = tasks.lock().map_err(|_| "子进程管理器不可用")?;
        if !active.load(Ordering::SeqCst) {
            return Err("插件已停用".into());
        }
        if operation == "process.spawn" {
            if tasks.len() >= GLOBAL
                || tasks.values().filter(|t| t.plugin == plugin).count() >= PER_PLUGIN
            {
                return Err("子进程记录已达上限，请先 dispose 已结束的进程".into());
            }
            let request = validate_spawn(args)?;
            let task = spawn(plugin, generation, request)?;
            if !active.load(Ordering::SeqCst) {
                return Err("插件已停用".into());
            }
            let mut id = fastrand::u64(1..(1u64 << 53));
            while tasks.contains_key(&id) {
                id = fastrand::u64(1..(1u64 << 53));
            }
            let pid = task.pid;
            tasks.insert(id, task);
            return Ok(json!({"id":id,"pid":pid}));
        }
        let id = args
            .get("id")
            .and_then(Value::as_u64)
            .ok_or("缺少子进程 ID")?;
        let task = tasks
            .get_mut(&id)
            .filter(|t| t.plugin == plugin && t.generation == generation)
            .ok_or("子进程不存在或已失效")?;
        match operation {
            "process.get" => task.snapshot(id),
            "process.read" => {
                let output = match args.get("stream").and_then(Value::as_str) {
                    Some("stdout") => &task.stdout,
                    Some("stderr") => &task.stderr,
                    _ => return Err("须指定 stdout 或 stderr".into()),
                };
                output.lock().map_err(|_| "输出缓冲不可用")?.read(args)
            }
            "process.write" => {
                let encoded = args
                    .get("data")
                    .and_then(Value::as_str)
                    .ok_or("缺少 base64 输入")?;
                if encoded.len() > CHUNK_BYTES.div_ceil(3) * 4 {
                    return Err("单次输入超过 16KiB".into());
                }
                let bytes = STANDARD.decode(encoded).map_err(|_| "输入须为 base64")?;
                if bytes.len() > CHUNK_BYTES {
                    return Err("单次输入超过 16KiB".into());
                }
                let length = bytes.len();
                task.stdin
                    .as_ref()
                    .ok_or("标准输入已关闭")?
                    .try_send(bytes)
                    .map_err(|error| match error {
                        mpsc::TrySendError::Full(_) => "标准输入缓冲已满，请稍后重试",
                        mpsc::TrySendError::Disconnected(_) => "标准输入已关闭",
                    })?;
                Ok(json!({"queuedBytes":length}))
            }
            "process.close-stdin" => {
                task.stdin = None;
                Ok(Value::Null)
            }
            "process.terminate" => {
                task.terminate()?;
                Ok(Value::Null)
            }
            "process.dispose" => {
                task.terminate()?;
                tasks.remove(&id);
                Ok(Value::Null)
            }
            _ => Err("未知子进程操作".into()),
        }
    }

    fn handle(value: &OwnedHandle) -> HANDLE {
        HANDLE(value.as_raw_handle())
    }
    unsafe fn owned(value: HANDLE) -> OwnedHandle {
        unsafe { OwnedHandle::from_raw_handle(value.0) }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    // CreateProcess receives one command line; encode each argument with Windows CRT quoting.
    fn quote(value: &str) -> String {
        let mut result = String::from("\"");
        let mut slashes = 0;
        for ch in value.chars() {
            if ch == '\\' {
                slashes += 1;
                continue;
            }
            result.extend(std::iter::repeat_n(
                '\\',
                slashes * if ch == '"' { 2 } else { 1 },
            ));
            slashes = 0;
            if ch == '"' {
                result.push('\\');
            }
            result.push(ch);
        }
        result.extend(std::iter::repeat_n('\\', slashes * 2));
        result.push('"');
        result
    }
    fn pipe(parent_reads: bool) -> AppResult<(OwnedHandle, OwnedHandle)> {
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            bInheritHandle: true.into(),
            ..Default::default()
        };
        let mut read = HANDLE::default();
        let mut write = HANDLE::default();
        unsafe { CreatePipe(&mut read, &mut write, Some(&attributes), 0) }
            .map_err(|e| e.to_string())?;
        let (read, write) = unsafe { (owned(read), owned(write)) };
        let (parent, child) = if parent_reads {
            (read, write)
        } else {
            (write, read)
        };
        unsafe { SetHandleInformation(handle(&parent), HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
            .map_err(|e| e.to_string())?;
        Ok((parent, child))
    }
    struct Attributes {
        _storage: Vec<usize>,
        list: LPPROC_THREAD_ATTRIBUTE_LIST,
    }
    impl Attributes {
        fn new(handles: &[HANDLE]) -> AppResult<Self> {
            let mut size = 0;
            let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut size) };
            let mut storage = vec![0usize; size.div_ceil(size_of::<usize>())];
            let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
            unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut size) }
                .map_err(|e| e.to_string())?;
            let attributes = Self {
                _storage: storage,
                list,
            };
            unsafe {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                    Some(handles.as_ptr().cast()),
                    size_of_val(handles),
                    None,
                    None,
                )
            }
            .map_err(|e| e.to_string())?;
            Ok(attributes)
        }
    }
    impl Drop for Attributes {
        fn drop(&mut self) {
            unsafe { DeleteProcThreadAttributeList(self.list) };
        }
    }

    fn read_output(pipe: OwnedHandle, output: Arc<Mutex<Output>>) -> AppResult<()> {
        std::thread::Builder::new()
            .name("nons-plugin-output".into())
            .spawn(move || {
                let mut file = File::from(pipe);
                let mut bytes = [0u8; 8192];
                loop {
                    match file.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(count) => {
                            if let Ok(mut state) = output.lock() {
                                state.append(&bytes[..count]);
                            } else {
                                break;
                            }
                        }
                        Err(error) => {
                            if error.kind() != std::io::ErrorKind::BrokenPipe {
                                if let Ok(mut state) = output.lock() {
                                    state.error = Some(error.to_string());
                                }
                            }
                            break;
                        }
                    }
                }
                if let Ok(mut state) = output.lock() {
                    state.eof = true;
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn spawn(plugin: &str, generation: u64, request: Spawn) -> AppResult<Task> {
        let job =
            unsafe { owned(CreateJobObjectW(None, PCWSTR::null()).map_err(|e| e.to_string())?) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                handle(&job),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of_val(&limits) as u32,
            )
        }
        .map_err(|e| e.to_string())?;
        let (input, child_input) = pipe(false)?;
        let (output, child_output) = pipe(true)?;
        let (error, child_error) = pipe(true)?;
        let inherited = [
            handle(&child_input),
            handle(&child_output),
            handle(&child_error),
        ];
        let attributes = Attributes::new(&inherited)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = inherited[0];
        startup.StartupInfo.hStdOutput = inherited[1];
        startup.StartupInfo.hStdError = inherited[2];
        startup.lpAttributeList = attributes.list;
        let program = wide(&request.program);
        let mut command = wide(
            &std::iter::once(request.program.as_str())
                .chain(request.args.iter().map(String::as_str))
                .map(quote)
                .collect::<Vec<_>>()
                .join(" "),
        );
        if command.len() > 32767 {
            return Err("进程命令行超过 Windows 长度限制".into());
        }
        let cwd = request.cwd.as_deref().map(wide);
        let mut info = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(program.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_SUSPENDED | CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT,
                None,
                cwd.as_ref()
                    .map(|s| PCWSTR(s.as_ptr()))
                    .unwrap_or(PCWSTR::null()),
                &startup.StartupInfo,
                &mut info,
            )
        }
        .map_err(|e| format!("启动程序失败：{e}"))?;
        let process = unsafe { owned(info.hProcess) };
        let thread = unsafe { owned(info.hThread) };
        // No guest instruction runs before job assignment; failures terminate the suspended root.
        if let Err(error) = unsafe { AssignProcessToJobObject(handle(&job), handle(&process)) } {
            let _ = unsafe { TerminateProcess(handle(&process), 1) };
            return Err(format!("进程树接管失败：{error}"));
        }
        drop((child_input, child_output, child_error, attributes));
        let stdout = Arc::new(Mutex::new(Output::default()));
        let stderr = Arc::new(Mutex::new(Output::default()));
        read_output(output, stdout.clone())?;
        read_output(error, stderr.clone())?;
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(8);
        std::thread::Builder::new()
            .name("nons-plugin-input".into())
            .spawn(move || {
                let mut input = File::from(input);
                for bytes in receiver {
                    if input.write_all(&bytes).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let task = Task {
            plugin: plugin.to_owned(),
            generation,
            job,
            process,
            pid: info.dwProcessId,
            terminated: false,
            stdin: Some(sender),
            stdout,
            stderr,
        };
        if unsafe { ResumeThread(handle(&thread)) } == u32::MAX {
            return Err("恢复子进程执行失败".into());
        }
        Ok(task)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, Instant};

        fn powershell() -> String {
            format!(
                "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
                std::env::var("SystemRoot").unwrap()
            )
        }
        fn start(tasks: &Mutex<BTreeMap<u64, Task>>, script: &str) -> Value {
            call(tasks, "test", 1, &AtomicBool::new(true), "process.spawn", &json!({"program":powershell(),"args":["-NoProfile","-NonInteractive","-Command",script]})).unwrap()
        }
        fn output(tasks: &Mutex<BTreeMap<u64, Task>>, id: u64, stream: &str) -> Vec<u8> {
            let start = Instant::now();
            loop {
                let result = call(
                    tasks,
                    "test",
                    1,
                    &AtomicBool::new(true),
                    "process.read",
                    &json!({"id":id,"stream":stream}),
                )
                .unwrap();
                if result["eof"] == true {
                    return STANDARD.decode(result["data"].as_str().unwrap()).unwrap();
                }
                assert!(start.elapsed() < Duration::from_secs(20));
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        #[test]
        fn roundtrips_input_and_separate_outputs_and_denies_old_generations() {
            let tasks = Mutex::new(BTreeMap::new());
            let result = start(&tasks,"$line=[Console]::ReadLine(); [Console]::Out.Write($line); [Console]::Error.Write('error')");
            let id = result["id"].as_u64().unwrap();
            assert!(call(
                &tasks,
                "other",
                1,
                &AtomicBool::new(true),
                "process.get",
                &json!({"id":id})
            )
            .is_err());
            assert!(call(
                &tasks,
                "test",
                2,
                &AtomicBool::new(true),
                "process.get",
                &json!({"id":id})
            )
            .is_err());
            call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.write",
                &json!({"id":id,"data":STANDARD.encode(b"hello\n")}),
            )
            .unwrap();
            call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.close-stdin",
                &json!({"id":id}),
            )
            .unwrap();
            assert_eq!(output(&tasks, id, "stdout"), b"hello");
            assert_eq!(output(&tasks, id, "stderr"), b"error");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                let state = call(
                    &tasks,
                    "test",
                    1,
                    &AtomicBool::new(true),
                    "process.get",
                    &json!({"id":id}),
                )
                .unwrap();
                if let Some(code) = state["exitCode"].as_i64() {
                    assert_eq!(code, 0);
                    break;
                }
                assert!(std::time::Instant::now() < deadline, "process did not exit");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        #[test]
        fn bounded_output_reports_dropped_bytes_and_rejects_invalid_cursor() {
            let mut output = Output::default();
            output.append(&vec![b'a'; OUTPUT_BYTES]);
            output.append(b"tail");
            assert_eq!(output.bytes.len(), OUTPUT_BYTES);
            let page = output.read(&json!({"cursor":0,"maxBytes":4})).unwrap();
            assert_eq!(page["dropped"], 4);
            assert_eq!(page["cursor"], 4);
            assert!(output.read(&json!({"cursor":OUTPUT_BYTES+5})).is_err());
        }

        #[test]
        fn terminates_descendants_even_after_root_exits() {
            use windows::Win32::System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            };
            let tasks = Mutex::new(BTreeMap::new());
            let result = start(&tasks,"$p=Start-Process -WindowStyle Hidden -FilePath ($env:SystemRoot+'\\System32\\WindowsPowerShell\\v1.0\\powershell.exe') -ArgumentList '-NoProfile -NonInteractive -Command Start-Sleep -Seconds 120' -PassThru; [Console]::Out.Write($p.Id)");
            let id = result["id"].as_u64().unwrap();
            let mut pid = None;
            for _ in 0..400 {
                let page = call(
                    &tasks,
                    "test",
                    1,
                    &AtomicBool::new(true),
                    "process.read",
                    &json!({"id":id,"stream":"stdout"}),
                )
                .unwrap();
                pid = String::from_utf8(STANDARD.decode(page["data"].as_str().unwrap()).unwrap())
                    .unwrap()
                    .parse::<u32>()
                    .ok();
                if pid.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            let child = unsafe {
                owned(
                    OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                        false,
                        pid.unwrap(),
                    )
                    .unwrap(),
                )
            };
            call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.terminate",
                &json!({"id":id}),
            )
            .unwrap();
            assert_eq!(
                unsafe { WaitForSingleObject(handle(&child), 5000) },
                WAIT_OBJECT_0
            );
        }

        #[test]
        fn enforces_record_limit_and_dispose_releases_slot() {
            let tasks = Mutex::new(BTreeMap::new());
            let mut ids = Vec::new();
            for _ in 0..PER_PLUGIN {
                ids.push(
                    start(&tasks, "Start-Sleep -Seconds 120")["id"]
                        .as_u64()
                        .unwrap(),
                );
            }
            assert!(call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.spawn",
                &json!({"program":powershell()})
            )
            .is_err());
            call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.dispose",
                &json!({"id":ids[0]}),
            )
            .unwrap();
            assert!(start(&tasks, "exit 0")["id"].is_u64());
        }

        #[test]
        fn preserves_empty_quoted_and_trailing_backslash_arguments() {
            assert_eq!(quote(""), "\"\"");
            assert_eq!(quote("a\\"), "\"a\\\\\"");
            assert_eq!(quote("a\"b"), "\"a\\\"b\"");
        }

        #[test]
        fn revoke_is_generation_scoped_and_kills_the_owned_job() {
            let processes = Processes::default();
            let result = start(&processes.tasks, "Start-Sleep -Seconds 120");
            let id = result["id"].as_u64().unwrap();
            let process = processes.tasks.lock().unwrap()[&id]
                .process
                .try_clone()
                .unwrap();
            processes.revoke("test", 2);
            assert_eq!(processes.tasks.lock().unwrap().len(), 1);
            processes.revoke("test", 1);
            assert!(processes.tasks.lock().unwrap().is_empty());
            assert_eq!(
                unsafe { WaitForSingleObject(handle(&process), 5000) },
                WAIT_OBJECT_0
            );
            assert!(call(
                &processes.tasks,
                "test",
                1,
                &AtomicBool::new(false),
                "process.spawn",
                &json!({"program":powershell()})
            )
            .is_err());
            assert!(processes.tasks.lock().unwrap().is_empty());
        }

        #[test]
        fn validates_absolute_paths_and_input_budget_before_spawning() {
            assert!(validate_spawn(&json!({"program":"powershell.exe"})).is_err());
            assert!(validate_spawn(&json!({"program":powershell(),"cwd":"relative"})).is_err());
            assert!(validate_spawn(&json!({"program":powershell(),"args":["a\u{0}b"]})).is_err());
            let tasks = Mutex::new(BTreeMap::new());
            let id = start(&tasks, "Start-Sleep -Seconds 120")["id"]
                .as_u64()
                .unwrap();
            assert!(call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.write",
                &json!({"id":id,"data":STANDARD.encode(vec![0;CHUNK_BYTES+1])})
            )
            .is_err());
            assert!(call(
                &tasks,
                "test",
                1,
                &AtomicBool::new(true),
                "process.read",
                &json!({"id":id,"stream":"stdout","maxBytes":CHUNK_BYTES+1})
            )
            .is_err());
        }
    }
}
