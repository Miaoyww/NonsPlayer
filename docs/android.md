# Android ARM64 试用版

当前提供 Windows 上直接构建的 Android ARM64 调试 APK，不需要 Android Studio，不运行 `tauri android dev` 或 Vite 开发服务器。最低 Android API 为 24（Android 7.0）；这只是构建下限，兼容性和播放仍需真机验收。

## 准备工具链

需要 Rust、Node/pnpm、Python 3.12+ 和 JDK 21。设置 `JAVA_HOME` 指向 JDK 目录；`scripts/android.ps1` 也会查找 `C:/Program Files/Java/jdk-21*`。脚本仅对子进程配置环境变量，不修改系统环境。

```powershell
python scripts/bootstrap-android.py
rustup target add aarch64-linux-android
```

引导脚本下载并校验 Google Android Command-line Tools，并使用 sdkmanager 安装 SDK 与 NDK。Android SDK/NDK 安装前由 `sdkmanager` 交互展示许可。工具保存在忽略目录 `.local/android`，不进入 Git。

Release CI 在同一个发布 workflow 中构建 Android ARM64 APK，并作为 GitHub Release 附件发布；Windows 与 Linux 构建不依赖 Android。CI 使用 `bootstrap-android.py --accept-licenses` 非交互接受 SDK 许可，本地默认仍保留交互确认。Release APK 使用固定的 upload keystore，调试 APK 仍使用本机 Android debug keystore。

Android 工程已提交在 `app/src-tauri/gen/android`；不要重新执行 `android init` 覆盖其中的原生源码路径与媒体服务配置。Android 原生能力源码在 `app/src-tauri/android/kotlin`。

## 直接构建和安装

```powershell
pnpm --dir app build:android
```

构建入口使用项目内的 SDK、NDK 27.2.12479018 和 Gradle 缓存，打包前端与 Rust 服务，并在编译前构建 QQMusic、GDStudio 与 Xiaobai 三套随包适配器。适配器资源嵌入 Android 原生库，首次启动时写入应用私有目录供通用 WASM Host 加载。只编译 ARM64，默认使用调试签名；首次构建需下载 Gradle、Maven 和缺失的 Rust 依赖。

输出：`app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`。虽然目录名为 `universal`，此构建入口的安装包仅包含 ARM64 原生库。验证后另存为 `.local/android/Nons-<版本>-android-arm64-debug.apk` 与带 `-compat` 后缀的同内容安装包，并为前者生成 SHA256 校验文件。

Android 启动器使用项目 Nons 图标，支持传统与自适应图标。调试包显式启用 V1/V2/V3 签名，仍使用同一台机器的 Android 调试证书；构建入口分别验证现代签名、JAR 签名、原生库 16 KB 对齐和 APK ZIP 对齐，全部通过后才导出。

## 正式签名构建

正式构建使用 `upload` 别名的 Java Keystore。密钥文件、密码和 `keystore.properties` 均不得提交到 Git；本地生成的材料保存在 `.local/android` 忽略目录。

```powershell
pnpm --dir app build:android:release
```

本地 Release 构建读取 `.local/android/android-signing.env`；CI 使用 `ANDROID_KEY_ALIAS`、`ANDROID_KEY_PASSWORD` 和 `ANDROID_KEY_BASE64` Secrets，在 runner 临时目录还原 keystore 后执行同一入口。Release APK 输出为 `.local/android/Nons-<版本>-android-arm64-release.apk`，调试 APK 的输出和安装方式保持不变。

调试 APK 使用本机 Android debug keystore，Release APK 使用 upload keystore，两者签名不同。调试版与 Release 版不能直接互相覆盖安装；切换前需卸载旧版本，卸载会清除应用数据。

重新生成 Android 图标：`pnpm --dir app icons:android`。此入口调用 Tauri 官方 `icon` CLI，使用 `docs/app-icon.png`，并将所有密度、圆形和自适应图标同步到 Android 工程。自适应前景四边各留出 1/6 的比例间距，将完整原图放入 108dp 图层的中央 72dp 可视区域，保持与桌面版相同的图案比例；普通与圆形入口共用此自适应配置，避免启动器裁切放大。

APK 校验检查 ARM64 原生库、16 KB 对齐与内嵌前端。WebAudio 解码和输出由 Android System WebView 执行，设备选择取决于 WebView 支持；原生媒体会话继续转发控制。

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
- 音频统一使用 WebAudio，Rust 通过本机流式桥接提供资源；网络安全配置仅为 `127.0.0.1` 放行 HTTP。调试构建允许开发服务器的 HTTP 连接。
- 隐藏桌面窗口按钮、自启和关闭行为设置；Android 不注册全局快捷键、不执行桌面 WebView 空闲销毁逻辑。
- 播放核心在加载、缓冲和播放期间启动原生 `mediaPlayback` 前台服务，通知显示当前歌名与歌手并可返回应用；持有 CPU 部分唤醒锁，暂停、停止、清空、错误和核心退出时释放。状态更新由浏览器反馈至 Rust 播放核心；后台计时与音频执行依赖 WebView。
- 已接入 Media3 `MediaSessionService`，自定义 `SimpleBasePlayer` 将通知栏、锁屏、耳机媒体按钮的播放／暂停、切歌、拖动进度和随机播放交给 Rust 命令队列；同步当前歌曲、歌手、专辑、封面、时长和进度，正常播放每秒校准一次。系统卡片通过 `CommandButton` 提供随机与收藏按钮；收藏复用统一音乐适配器，独立异步任务串行处理，未登录或不支持收藏时禁用，并向应用发送失效事件刷新状态。暂停后退出前台并释放 CPU 唤醒锁，保留媒体会话恢复入口；不提供外部队列替换、语音搜索点歌、Android Auto 曲库浏览或进程被杀后的播放恢复。卡片配色、波浪进度条、设备选择和最终按钮布局由 Android 版本与厂商系统决定。
- WebAudio 后台和息屏播放需要重新进行真机验收；音频焦点与耳机拔出事件尚未接入，长时间锁屏、厂商省电策略和电话打断仍需进一步验收。MediaSession 的实际系统界面、耳机按钮及设备兼容性需要独立真机验收。
- 系统歌词接入尚未完成验收：已复用 AMLL 解析并向媒体元数据发布当前行、翻译和音译，补充 vivo 歌词能力标记；修复仅改变 `extras` 不触发 Media3 元数据更新的问题，并在行间间隔保留当前行。2026-10-14 真机反馈中，封面已正常显示，但 vivo 系统卡片仍提示未提供歌词；不能视为已支持 vivo 卡片歌词。调试构建可通过 `android_media_status` 检查原生歌词行数、当前行和平台元数据长度，不输出歌词正文或账号信息。
- Android 本地音乐通过 `tauri-plugin-android-fs` 接入 SAF 系统目录选择器和持久授权，递归扫描所选目录；启动、回到前台和手动重新扫描时更新，不依赖文件提供器的变化通知。目录授权失效或读取失败时保留已有索引并显示错误；移除目录撤销本地音乐的访问并清理来源关系，原文件保留。MediaStore 自动发现、插件目录授权和剪贴板监听尚未接入。
- Android Release CI 生成固定 upload keystore 签名 APK 并随 GitHub Release 分发；Google Play 上架、Play App Signing、AAB 发布和真机性能结论仍需单独验收。最低版本、登录、切歌、歌词、弱网和安装升级均需真机验收。

## 真机回归

WebAudio 的前台播放、连续切歌、后台息屏与暂停恢复均需在目标设备验收。

本地音乐还需验证以下行为：

- 添加含子目录的音乐文件夹，确认音频标签、外部／内嵌封面、CUE 分轨及同名 LRC 正常读取。
- 强制结束应用并重新打开，确认无需再次选择目录即可恢复扫描和播放。
- 播放普通音频与 NCM，拖动至开头、中间和末尾，连续切歌并检查预加载。普通音频直接从授权文件描述符读取，NCM 沿用有界临时解码文件。
- 在后台添加、替换或删除音乐，回到前台后确认曲库更新；前台修改文件后手动重新扫描。
- 从系统撤销访问或卸载存储卷，确认扫描报告错误并保留原索引；重新授权后恢复读取。移除文件夹后确认旧播放资源失效且原文件未删除。

真机回归入口（需要连接一台已解锁设备、打开调试 APK 并准备有可播歌曲的队列；结束后暂停播放）：

```powershell
node scripts/check-android-playback.mjs 24 15
```

两个参数分别为切歌次数（1–100）与后台息屏等待秒数（1–60）。脚本检查曲目进度、音频欠载／时钟修正日志，以及暂停后的服务和唤醒锁释放；不输出账号信息、音频地址或完整队列。

## 参考资料

- [Android 媒体播放前台服务](https://developer.android.com/develop/background-work/services/fgs/service-types#media)
- [Tauri Android 环境要求](https://v2.tauri.app/start/prerequisites/)
- [Android Keystore](https://developer.android.com/privacy-and-security/keystore)
