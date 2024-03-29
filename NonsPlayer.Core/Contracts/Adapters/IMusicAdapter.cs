﻿using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IMusicAdapter
{
    Task<Music> GetMusicAsync(object content);
    
    Task<Music[]> GetMusicListAsync(object content);

    Music[] GetMusicList(object content);

    Music GetMusic(object content);
}