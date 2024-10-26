﻿using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Adapters;


namespace NonsPlayer.Core.Contracts.Models.Music;

public interface IPlaylist : IMusicModel
{
    [JsonPropertyName("create_time")] public DateTime CreateTime { get; set; }
    [JsonPropertyName("creator")] public string Creator { get; set; }
    [JsonPropertyName("description")] public string Description { get; set; }
    [JsonPropertyName("music_track_ids")] public string[] MusicTrackIds { get; set; }
    [JsonPropertyName("tags")] public string[] Tags { get; set; }
    [JsonIgnore] IAdapter Adapter { get; set; }
    public List<IMusic> Musics { get; set; }
    public bool IsInitialized { get; set; }
    public int MusicsCount => MusicTrackIds.Length;
    public string CacheId => Id + "_playlist";

    /// <summary>
    /// 初始化歌单
    /// </summary>
    Task InitializePlaylist();

    Task InitializeMusics();

    Task InitializeTracks();
}