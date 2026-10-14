# Android ARM64 试用版

当前提供 Windows 上直接构建的 Android ARM64 调试 APK，不需要 Android Studio，不运行 `tauri android dev` 或 Vite 开发服务器。最低 Android API 为 24（Android 7.0）；这只是构建下限，兼容性和播放仍需真机验收。

## 准备工具链

需要 Rust、Node/pnpm、Python 3.12+ 和 JDK 21。设置 `JAVA_HOME` 指向 JDK 目录；`scripts/android.ps1` 也会查找 `C:/Program Files/Java/jdk-21*`。脚本仅对子进程配置环境变量，不修改系统环境。

```powershell
python scripts/bootstrap-android.py
rustup target add aarch64-linux-android
```

引导脚本下载并校验 Google Android Command-line Tools 与 GStreamer 1.28.7 官方 Android SDK，只提取 ARM64 运行时。Android SDK/NDK 安装前由 `sdkmanager` 交互展示许可。工具保存在忽略目录 `.local/android`，不进入 Git。

Release CI 在版本标签上自动构建并上传 ARM64 试用 APK，流程见 [GitHub Release CI](release.md)。CI 使用 `bootstrap-android.py --accept-licenses` 非交互接受 SDK 许可，本地默认仍保留交互确认。CI 的临时调试证书与本地或其他 CI 运行不同，签名冲突时需要卸载旧包（会清除应用数据）；正式签名与跨版本覆盖升级仍待接入。

下载较慢时，可手动下载 [GStreamer 安装包](https://gstreamer.freedesktop.org/data/pkg/android/1.28.7/gstreamer-1.0-android-universal-1.28.7.tar.xz)，不用解压，放到 `.local/android/gstreamer-1.0-android-universal-1.28.7.tar.xz`，再执行引导脚本。已有完整包通过官方 SHA256 校验后复用。

Android 工程已提交在 `app/src-tauri/gen/android`；不要重新执行 `android init` 覆盖其中的原生源码路径和 GStreamer 初始化。Android 原生能力源码在 `app/src-tauri/android/kotlin`，音频插件选择在 `app/src-tauri/android/audio`。

## 直接构建和安装

```powershell
pnpm --dir app build:android
```

构建入口使用项目内的 SDK、NDK 27.2.12479018 和 Gradle 缓存，先用官方 NDK makefile 链接 GStreamer 音频插件，再打包前端和 Rust 服务。只编译 ARM64，使用调试签名；首次构建需下载 Gradle、Maven 和缺失的 Rust 依赖。Android 配置不会构建桌面随包插件。

输出：`app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`。虽然目录名为 `universal`，此构建入口的安装包仅包含 ARM64 原生库。验证后另存为 `.local/android/Nons-<版本>-android-arm64-debug.apk` 与带 `-compat` 后缀的同内容安装包，并为前者生成 SHA256 校验文件。

Android 启动器使用项目 Nons 图标，支持传统与自适应图标。调试包显式启用 V1/V2/V3 签名，仍使用同一台机器的 Android 调试证书；构建入口分别验证现代签名、JAR 签名、原生库 16 KB 对齐和 APK ZIP 对齐，全部通过后才导出。

重新生成 Android 图标：`pnpm --dir app icons:android`。此入口调用 Tauri 官方 `icon` CLI，使用 `docs/app-icon.png`，并将所有密度、圆形和自适应图标同步到 Android 工程。自适应前景四边各留出 1/6 的比例间距，将完整原图放入 108dp 图层的中央 72dp 可视区域，保持与桌面版相同的图案比例；普通与圆形入口共用此自适应配置，避免启动器裁切放大。

原生运行时显式包含 GStreamer Base；APK 校验还检查 Rust 库需要的 GStreamer/GLib 动态符号是否由打包的运行时导出，防止编译通过但 Android 动态加载器启动失败。

Android 音频输出跟随系统路由，不启动桌面音频设备监视器。播放运行时包含渐变链所需的 `audiofx` 与 `audiobuffersplit`；APK 校验同时检查必需插件的注册符号，避免安装后才发现缺少元素工厂。

把 APK 传到手机，允许对应文件管理器安装未知来源应用；或开启 USB 调试后执行：

```powershell
.local/android/sdk/platform-tools/adb.exe install -r app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

检查安装包原生库和内嵌前端：

```powershell
python scripts/verify-android-apk.py app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## 当前边界

- 窗口采用边到边显示，原生同步系统栏、挖孔安全区与键盘占用；WebView 不再重复消费系统栏留白。全屏播放器及歌词页采用沉浸模式，边缘滑动临时唤出系统栏，退出播放器后恢复；系统栏图标跟随应用明暗主题。旋转、手势／三键导航及各版本 WebView 的实际表现仍需真机验证。
- 复用统一音乐适配器、网易云业务、Rust 播放核心、歌词及缓存服务。Android 账号凭据采用 Keystore 密钥与 AES-GCM 加密存储，禁止系统备份复制密文到其他设备。
- GStreamer 通过 Java glue 在 Activity 启动时初始化；音频 HTTP 源在 Android 使用 WebPKI 证书根，与宿主请求保持一致，无额外 TLS JNI 运行时。
- 隐藏桌面窗口按钮、自启和关闭行为设置；Android 不注册全局快捷键、不执行桌面 WebView 空闲销毁逻辑。
- 播放核心在加载、缓冲和播放期间启动原生 `mediaPlayback` 前台服务，通知显示当前歌名与歌手并可返回应用；持有 CPU 部分唤醒锁，暂停、停止、清空、错误和核心退出时释放。状态更新来自 Rust 播放核心，不依赖后台 WebView 的计时器。音频输出使用设备时钟，不套用桌面 WASAPI 的 30ms 输出限速；切歌等待音频预滚及网络缓冲完成后再启动输出。
- 已接入 Media3 `MediaSessionService`，自定义 `SimpleBasePlayer` 将通知栏、锁屏、耳机媒体按钮的播放／暂停、切歌、拖动进度和随机播放交给 Rust 命令队列；同步当前歌曲、歌手、专辑、封面、时长和进度，正常播放每秒校准一次。系统卡片通过 `CommandButton` 提供随机与收藏按钮；收藏复用统一音乐适配器，独立异步任务串行处理，未登录或不支持收藏时禁用，并向应用发送失效事件刷新状态。暂停后退出前台并释放 CPU 唤醒锁，保留媒体会话恢复入口；不提供外部队列替换、语音搜索点歌、Android Auto 曲库浏览或进程被杀后的播放恢复。卡片配色、波浪进度条、设备选择和最终按钮布局由 Android 版本与厂商系统决定。
- 后台和息屏播放已做短时真机回归，仍属试用版；音频焦点与耳机拔出事件尚未接入，长时间锁屏、厂商省电策略和电话打断仍需进一步验收。MediaSession 的实际系统界面、耳机按钮及设备兼容性需要独立真机验收。
- 系统歌词接入尚未完成验收：已复用 AMLL 解析并向媒体元数据发布当前行、翻译和音译，补充 vivo 歌词能力标记；修复仅改变 `extras` 不触发 Media3 元数据更新的问题，并在行间间隔保留当前行。2026-10-14 真机反馈中，封面已正常显示，但 vivo 系统卡片仍提示未提供歌词；不能视为已支持 vivo 卡片歌词。调试构建可通过 `android_media_status` 检查原生歌词行数、当前行和平台元数据长度，不输出歌词正文或账号信息。
- Android 本地音乐的 MediaStore/SAF、插件目录授权和剪贴板监听尚未接入；当前不能视为完整移动端版本。
- 已接入 Android Release CI；尚无 Google Play 上架、正式发布签名或真机性能结论。最低版本、登录、切歌、歌词、弱网和安装升级均需真机验收。

## 真机验证（2026-10-13）

在 vivo V2502DA、Android 17、ARM64 上，通过 USB 覆盖安装和冷启动验证：原先 `gst_base_src_get_type` 缺失导致的启动闪退已复现并修复；修复包启动后进程存活，主界面及登录二维码加载成功。此验证不代表完成播放、后台行为或性能验收。

同一设备上进一步复现并修复播放核心初始化失败：先是音频设备监视器 `Failed to start`，随后是缺少 `audiobuffersplit`。修复包覆盖安装后，在线歌曲进入 `playing`，继续播放约 2.5 秒后进度增长至 2044 ms；暂停后两次读取均为 2066 ms，错误字段为空。此项验证覆盖前台播放状态与暂停/继续操作，未验证实际听感、后台播放、完整歌曲或弱网恢复。

同一设备上复现切歌输出异常：连续切换 6 首在线歌曲时，日志记录 40 次 `correct clock skew`、3 次 `No segment available`；随后捕获进度停在约 30ms。修复输出时钟、供给节奏及预滚启动顺序后，最终包连续切歌 24 次（其中 6 次在加载后立即暂停、100ms 后继续）均正常推进；后台息屏等待 15 秒，前后进度增长 15920ms，上述两类日志均为 0。暂停后前台服务停止、活动 CPU 唤醒锁释放。这是状态与日志回归，不代表完成实际听感、长时间后台、蓝牙切换或弱网验收。

真机回归入口（需要连接一台已解锁设备、打开调试 APK 并准备有可播歌曲的队列；结束后暂停播放）：

```powershell
node scripts/check-android-playback.mjs 24 15
```

两个参数分别为切歌次数（1–100）与后台息屏等待秒数（1–60）。脚本检查曲目进度、音频欠载／时钟修正日志，以及暂停后的服务和唤醒锁释放；不输出账号信息、音频地址或完整队列。

## 参考资料

- [Android 媒体播放前台服务](https://developer.android.com/develop/background-work/services/fgs/service-types#media)
- [Tauri Android 环境要求](https://v2.tauri.app/start/prerequisites/)
- [GStreamer Android 集成](https://gstreamer.freedesktop.org/documentation/installing/for-android-development.html)
- [Android Keystore](https://developer.android.com/privacy-and-security/keystore)
