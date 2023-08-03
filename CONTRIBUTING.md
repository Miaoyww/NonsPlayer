# Welcome to Nonsplayer contributing guide!

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

## Getting Started

### What you can do for this repo?

**If you have any problem, you can Contact Me.For Chinese speakers, please use Chinese for communication.**

- Bug fix
    + Firstly, if any issue opens, you can review and then try to solve it.
      E.g if any one opened a bug issue, you can have a try to attempt to reproduce the bug.
      Debug NonsPlayer and understand why it happens`.
      Use your knowledge to fix it.
    + Additionally, consider using lib like `CacheManager` to optimize performance.
    + Finally, create a pull request and wait for merging.
- Feature Apply
    + Similar to bug fixes, check and try to apply new features or improvements.

- Follow my step

+ NonsPlayer is still under development, so we really need you help!Come and join us!
+ As you can see, many parts of nonsplayer aren't perfect such as key bindings and Ui.
+ However, **just do what You Want To Do,it's OK**.If you have any new idea,feel free to apply it!
    - What you can do now?
        + Key bindings part
        + Settings backend part
        + Search backend part
        + Multiple platforms' api part
        + Lyric part
        + Android remote control part
        + Performance optimization.
        + And more.
