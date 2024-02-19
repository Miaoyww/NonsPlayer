<div align="center">
<!--![Alt](exp.png "exp")-->

<img src="NonsPlayer-Icon.png" alt="图标" Height="128" Width="128">

# NonsPlayer

![.net](https://img.shields.io/badge/C%23-.net7.0-orange)
![Windows](https://img.shields.io/badge/Windows-10%2B-orange)
![license](https://img.shields.io/github/license/Miaoywww/NeteaseCloudMusicControl)

A music player that can be controlled remotely

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

Find `NonsPlayer.exe` and open it, then you can enjoy time!

After installed the cer, double click the `msix` file and press Install Button and then You can use it!

**Require[.net7.0](https://dotnet.microsoft.com/zh-cn/download/dotnet/7.0)**


**Windows 10 needs [Fluent Icons](https://learn.microsoft.com/zh-cn/windows/apps/design/downloads/#fonts)，NonsPlayer will automatically install it for you in the future, but for now you need manually install it**

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
    - `Apis` - The apis we are using now, but we don't suggest to add any other platforms' api, we will provide a adapter to use soon
    - `Account` - Account system here
    - `Player` - Player system here

Welcome **Issues** and **Pull Request**!

# 📜Open Source License

Copyright Miaomiaoywww 2022.  

Distributed under the terms of
the [MIT license](https://github.com/Miaoywww/NeteaseCloudMusicControl/blob/master/LICENSE.txt).

# 💡Source of inspiration

- [Spotify](https://www.spotify.com/)
- [Apple Music](https://music.apple.com)
- [YesPlayMusic](https://github.com/qier222/YesPlayMusic)
- [网易云音乐](https://music.163.com/)
- [QQ音乐](https://y.qq.com/)
- [BiliBili客户端](https://app.bilibili.com/)
- [Apple Music-like Lyrics](https://github.com/Steve-xmh/applemusic-like-lyrics) 
# Thanks for

- [Zhuym](https://github.com/Zhuym07) providing the idea of Icon
- [GooGuJiang](https://github.com/GooGuJiang) providing help with Ui and Icon
- [NeteaseCloudMusicApi](https://github.com/Binaryify/NeteaseCloudMusicApi) for providing the api of NeteaseCloudMusic
- [StarWard](https://github.com/Scighost/Starward) A fantastic WinUi3 project! For providing some parts code logic, with some code being modified and adapted.
- [Apple Music-like Lyrics](https://github.com/Steve-xmh/applemusic-like-lyrics) A fantastic apple music-like lyrics display component library! For providing the lyric service!
  
<div align="center">
<image src="https://resources.jetbrains.com/storage/products/company/brand/logos/jb_beam.svg"></image>
<div>
Special thanks to <a href=https://jb.gg/OpenSourceSupport>JetBrains</a> for providing great support for this project!
</div>
</div>