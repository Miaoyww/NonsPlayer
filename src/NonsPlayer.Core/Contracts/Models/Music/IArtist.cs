﻿using NonsPlayer.Core.Contracts.Adapters;
using System.Text.Json.Serialization;
using NonsPlayer.Core.Contracts.Models;

namespace NonsPlayer.Core.Contracts.Models.Music;

public interface IArtist : IMusicModel
{
    [JsonPropertyName("desc")] public string Description { get; set; }
    [JsonPropertyName("musics")] public HashSet<IMusic> Songs { get; set; }
    [JsonPropertyName("music_count")] public int MusicCount { get; set; }
    [JsonPropertyName("trans")] public string Trans { get; set; }
    [JsonIgnore] IAdapter Adapter { get; set; }

}