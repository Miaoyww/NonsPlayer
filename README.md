<div align="center">
<!--![Alt](exp.png "exp")-->

<img src="NonsPlayer-Icon.png" alt="图标" Height="128" Width="128">

# NonsPlayer

![.net](https://img.shields.io/badge/C%23-.net6.0-orange)
![Windows](https://img.shields.io/badge/Windows-10%2B-orange)
![license](https://img.shields.io/github/license/Miaoywww/NeteaseCloudMusicControl)

A music player that allows you to remote control it.

![Alt](https://repobeats.axiom.co/api/embed/104248b2c1f2c27f8f5b29df5ab1ab2a4269ed96.svg "Repobeats analytics image")

***

</div>

**English** | [中文](https://github.com/Miaoyww/NonsPlayer/blob/master/README-CN.md)

# UI Development Progress

For details: [Latest Release](https://github.com/Miaoyww/NonsPlayer/releases/latest)

# ⭐Highlights

- Allows you to play music **On Multiple Platforms** (Windows play, Android control)
- Allows you to play music **From Multiple Platforms** and local music
- **Beautiful | Simple** lyrics display
- **Beautiful UI**, easy to use
- **Small Memory Footprint**, almost **No Impact** on performance
- **Stable Updates**

# 📦️Download

Open the [Latest Release](https://github.com/Miaoywww/NeteaseCloudMusicControl/releases) Page, download the latest
version, unzip it.

You can see a `NonsPlayer_x.x.x.x_xxx.msix` file and a `NonsPlayer_x.x.x.x_xxx.cer` file.
Double Click the cer, select `Install Certificate` and press Next Button.Select `Local Machine` and next
select `Automatically select the certificate store based...` and press Next Button.Then Finish installing the
certificate.

After installed the cer, double click the `msix` file and press Install Button and then You can use it!

**Require[.net6.0](https://dotnet.microsoft.com/zh-cn/download/dotnet/6.0)**

# 🧭Development

## Environment

+ Windows 10 1809 or later
+ 8GB RAM or more

## Step

+ Install an IDE like [Jetbrains Rider](https://www.jetbrains.com/rider/)
  or [Visual Studio](https://visualstudio.microsoft.com/)
+ Install [Windows App SDK](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/)
+ Clone this repo
+ Open `NonsPlayer.sln`

## Structure

I will introduce the local structure of this project.Others like Mvvm
and so on will not be introduced here.

+ `NonsPlayer` - The main project
    - `Components` - The components of the main project, like playerBar, playQueueCard are here
    - `Cache` - The whole cache system, please get and access data here.Basic usage see
      `Components`-> `ViewModels` -> `PlaylistCardViewModel.cs`
+ `NonsPlayer.Core` - The core project
    - `Apis` - The apis of the core project, you can create other platforms apis here
    - `Account` - Account system here
    - `Player` - Player system here

Welcome **Issues** and **Pull Request**!

# 📜Open Source License

Copyright Miaomiaoywww 2022.

Distributed under the terms of
the [MIT license](https://github.com/Miaoywww/NeteaseCloudMusicControl/blob/master/LICENSE.txt).

# Thanks for

- [Zhuym](https://github.com/Zhuym07) provide the idea of Icon
- [GooGuJiang](https://github.com/GooGuJiang) provide help with Ui and Icon

# 💡Source of inspiration

- [Spotify](https://www.spotify.com/)
- [Apple Music](https://music.apple.com)
- [YesPlayMusic](https://github.com/qier222/YesPlayMusic)
- [网易云音乐](https://music.163.com/)
- [QQ音乐](https://y.qq.com/)
- [BiliBili客户端](https://app.bilibili.com/)