<div align="center">
<!--![Alt](exp.png "exp")-->

<img src="NonsPlayer-Icon.png" alt="图标" Height="128" Width="128">

# NonsPlayer

![.net](https://img.shields.io/badge/C%23-.net8.0-orange)
![Windows](https://img.shields.io/badge/Windows-10%2B-orange)
![license](https://img.shields.io/github/license/Miaoywww/NeteaseCloudMusicControl)

一个可以远程控制的音乐播放器

![Alt](https://repobeats.axiom.co/api/embed/104248b2c1f2c27f8f5b29df5ab1ab2a4269ed96.svg "Repobeats analytics image")

***

</div>

[English](https://github.com/Miaoyww/NonsPlayer/blob/master/README.md) | **中文**

# UI开发进度

详情见: [Latest Release](https://github.com/Miaoyww/NonsPlayer/releases/latest)

# ⭐亮点

- 可以**多平台同步播放**(Windows播放, Android控制)
- 可播放**多个平台内的音乐以及本地音乐**
- **漂亮|简洁**的歌词显示
- 界面美观, 操作简单
- 内存占用小, 对性能几乎没有影响
- 保持稳定更新

# 📦️下载

打开 [Latest Release](https://github.com/Miaoywww/NeteaseCloudMusicControl/releases) 页面, 下载最新版本,并解压

找到 `NonsPlayer.exe` 文件, 打开它即可！

**需要[.net8.0](https://dotnet.microsoft.com/zh-cn/download/dotnet/8.0)**

**Windows 10版本需要额外下载[Fluent Icons](https://learn.microsoft.com/zh-cn/windows/apps/design/downloads/#fonts)字体库，在未来的版本中会自动为您安装，但现在您需要手动安装它**
# 🧭开发指南

## 环境

+ Windows 10 1809 以上版本
+ 8GB RAM or more

## 步骤

+ 安装IDE [Jetbrains Rider](https://www.jetbrains.com/rider/)
  or [Visual Studio](https://visualstudio.microsoft.com/)
+ 安装 [Windows App SDK](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/)
+ Clone本项目
+ 打开`NonsPlayer.sln`文件

## 结构

我将介绍这个项目的本地结构。
其他如 Mvvm 等其他结构将不在此介绍。

+ `NonsPlayer` - 主项目
    - `Components` - 项目的组件库, 如播放栏, 播放列表卡片都在这里
    - `Cache` - 缓存系统, 请在此处获取以及注册数据, 基础使用请见
      `Components`-> `ViewModels` -> `PlaylistCardViewModel.cs`
+ `NonsPlayer.Core` - 核心项目
    - `Apis` - 目前使用的Api在这里，不过不建议向其添加其它平台的内容，后续会提供adapter供多平台接口使用
    - `Account` - 用户系统
    - `Player` - 播放器系统

# 📜开源许可

Copyright Miaoyww 2022-2024.

Distributed under the terms of
the [GPL-3.0 license](https://github.com/Miaoywww/NeteaseCloudMusicControl/blob/master/LICENSE.txt).

# 💡灵感来源

- [Spotify](https://www.spotify.com/)
- [Apple Music](https://music.apple.com)
- [YesPlayMusic](https://github.com/qier222/YesPlayMusic)
- [网易云音乐](https://music.163.com/)
- [QQ音乐](https://y.qq.com/)
- [BiliBili客户端](https://app.bilibili.com/)
- [Apple Music-like Lyrics](https://github.com/Steve-xmh/applemusic-like-lyrics) 

# 特别鸣谢

- [Zhuym](https://github.com/Zhuym07) 为图标设计提供帮助
- [GooGuJiang](https://github.com/GooGuJiang) 为图标设计、UI设计提供帮助
- [NeteaseCloudMusicApi](https://github.com/Binaryify/NeteaseCloudMusicApi) 提供网易云音乐Api
- [Starward](https://github.com/Scighost/Starward) 一个出色的WinUi3项目！提供了部分代码逻辑，部分代码经过了修改和调整。
- [Apple Music-like Lyrics](https://github.com/Steve-xmh/applemusic-like-lyrics) 提供出色的歌词服务!
  
<div align="center">
<image src="https://resources.jetbrains.com/storage/products/company/brand/logos/jb_beam.svg"></image>
<div>
特别感谢 <a href=https://jb.gg/OpenSourceSupport>JetBrains</a> 为本项目提供的大力支持
</div>
</div>
