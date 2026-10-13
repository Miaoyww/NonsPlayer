# Android ARM64 试用版

当前提供 Windows 上直接构建的 Android ARM64 调试 APK，不需要 Android Studio，不运行 `tauri android dev` 或 Vite 开发服务器。最低 Android API 为 24（Android 7.0）；这只是构建下限，兼容性和播放仍需真机验收。

## 准备工具链

需要 Rust、Node/pnpm、Python 3.12+ 和 JDK 21。设置 `JAVA_HOME` 指向 JDK 目录；`scripts/android.ps1` 也会查找 `C:/Program Files/Java/jdk-21*`。脚本仅对子进程配置环境变量，不修改系统环境。

```powershell
python scripts/bootstrap-android.py
rustup target add aarch64-linux-android
```

引导脚本下载并校验 Google Android Command-line Tools 与 GStreamer 1.28.7 官方 Android SDK，只提取 ARM64 运行时。Android SDK/NDK 安装前由 `sdkmanager` 交互展示许可。工具保存在忽略目录 `.local/android`，不进入 Git。

下载较慢时，可手动下载 [GStreamer 安装包](https://gstreamer.freedesktop.org/data/pkg/android/1.28.7/gstreamer-1.0-android-universal-1.28.7.tar.xz)，不用解压，放到 `.local/android/gstreamer-1.0-android-universal-1.28.7.tar.xz`，再执行引导脚本。已有完整包通过官方 SHA256 校验后复用。

Android 工程已提交在 `app/src-tauri/gen/android`；不要重新执行 `android init` 覆盖其中的原生源码路径和 GStreamer 初始化。Android 原生能力源码在 `app/src-tauri/android/kotlin`，音频插件选择在 `app/src-tauri/android/audio`。

## 直接构建和安装

```powershell
pnpm --dir app build:android
```

构建入口使用项目内的 SDK、NDK 27.2.12479018 和 Gradle 缓存，先用官方 NDK makefile 链接 GStreamer 音频插件，再打包前端和 Rust 服务。只编译 ARM64，使用调试签名；首次构建需下载 Gradle、Maven 和缺失的 Rust 依赖。Android 配置不会构建桌面随包插件。

输出：`app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`。虽然目录名为 `universal`，此构建入口的安装包仅包含 ARM64 原生库。验证后另存为 `.local/android/Nons-<版本>-android-arm64-debug.apk` 与带 `-compat` 后缀的同内容安装包，并为前者生成 SHA256 校验文件。

Android 启动器使用项目 Nons 图标，支持传统与自适应图标。调试包显式启用 V1/V2/V3 签名，仍使用同一台机器的 Android 调试证书；构建入口分别验证现代签名、JAR 签名、原生库 16 KB 对齐和 APK ZIP 对齐，全部通过后才导出。

把 APK 传到手机，允许对应文件管理器安装未知来源应用；或开启 USB 调试后执行：

```powershell
.local/android/sdk/platform-tools/adb.exe install -r app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

检查安装包原生库和内嵌前端：

```powershell
python scripts/verify-android-apk.py app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## 当前边界

- 复用统一音乐适配器、网易云业务、Rust 播放核心、歌词及缓存服务。Android 账号凭据采用 Keystore 密钥与 AES-GCM 加密存储，禁止系统备份复制密文到其他设备。
- GStreamer 通过 Java glue 在 Activity 启动时初始化；音频 HTTP 源在 Android 使用 WebPKI 证书根，与宿主请求保持一致，无额外 TLS JNI 运行时。
- 隐藏桌面窗口按钮、自启和关闭行为设置；Android 不注册全局快捷键、不执行桌面 WebView 空闲销毁逻辑。
- 仅用于前台试用。后台播放前台服务、锁屏/通知栏媒体控制、音频焦点与耳机事件尚未接入，不能承诺锁屏或切到其他应用后持续播放。
- Android 本地音乐的 MediaStore/SAF、插件目录授权和剪贴板监听尚未接入；当前不能视为完整移动端版本。
- 尚无 Google Play 上架、正式发布签名、Android CI 或真机性能结论。最低版本、登录、切歌、歌词、弱网和安装升级均需真机验收。

## 参考

- [Tauri Android 环境要求](https://v2.tauri.app/start/prerequisites/)
- [GStreamer Android 集成](https://gstreamer.freedesktop.org/documentation/installing/for-android-development.html)
- [Android Keystore](https://developer.android.com/privacy-and-security/keystore)
