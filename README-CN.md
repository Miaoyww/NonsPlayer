<div align="center">
<!--![Alt](exp.png "exp")-->

<img src="NonsPlayer-Icon.png" alt="图标" Height="128" Width="128">

# NonsPlayer

![.net](https://img.shields.io/badge/C%23-.net6.0-orange)
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

你可以看到解压后目录中有 `NonsPlayer_x.x.x.x_xxx.msix` 文件和 `NonsPlayer_x.x.x.x_xxx.cer` 文件
双击`cer`文件, 选择 `安装证书` 然后点击下一步.选择 `本地计算机` 然后下一步
选择 `根据证书类型自动..` 接着下一步, 然后完成即可.
双击打开`msix` 文件, 然后点击安装即可.

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
    - `Apis` - 其他平台的API都在这里, 请在这里创建新的API
    - `Account` - 用户系统
    - `Player` - 播放器系统

# 📜开源许可

Copyright Miaomiaoywww 2022.

Distributed under the terms of
the [MIT license](https://github.com/Miaoywww/NeteaseCloudMusicControl/blob/master/LICENSE.txt).

# 特别鸣谢

- [Zhuym](https://github.com/Zhuym07) 为图标设计提供帮助
- [GooGuJiang](https://github.com/GooGuJiang) 为图标设计、UI设计提供帮助

# 💡灵感来源

- [Spotify](https://www.spotify.com/)
- [Apple Music](https://music.apple.com)
- [YesPlayMusic](https://github.com/qier222/YesPlayMusic)
- [网易云音乐](https://music.163.com/)
- [QQ音乐](https://y.qq.com/)
- [BiliBili客户端](https://app.bilibili.com/)
