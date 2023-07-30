﻿using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class Playlist
{
    [JsonPropertyName("create_time")] public DateTime CreateTime;

    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds;

    [JsonPropertyName("tags")] public string[] Tags;

    [JsonPropertyName("name")] public string Name;

    [JsonPropertyName("id")] public long? Id;
    public string CacheId => Id.ToString() + "_playlist";
    public string CacheCoverId => Id.ToString() + "_playlist_cover";
    public string CacheMiddleCoverId => Id.ToString() + "_playlist_middle_cover";
    public string CacheSmallCoverId => Id.ToString() + "_playlist_small_cover";

    [JsonPropertyName("pic_url")] public string CoverUrl;
    public string SmallCoverUrl => CoverUrl + "?param=40y40";
    public string MiddleCoverUrl => CoverUrl + "?param=200y200";
    [JsonPropertyName("description")] public string Description;

    [JsonPropertyName("creator")] public string Creator;

    [JsonPropertyName("musics_count")] public int MusicsCount => MusicTrackIds.Length;

    [JsonPropertyName("musics")] public Music[] Musics;
    public Playlist This => this;

    public async Task LoadAsync()
    {
        if (Id == null)
        {
            throw new PlaylistIdNullException("歌单Id为空, 请在调用此函数前先设置Id");
        }

        await LoadAsync(Id);
    }

    public async Task LoadAsync(long? in_id)
    {
        Id = in_id;
        var (playlistDetail, apiRequestElapsed) =
            await Tools.MeasureExecutionTimeAsync(Apis.Playlist.Detail(Id, Nons.Instance));
        Debug.WriteLine($"歌单Api请求({Id})所用时间: {apiRequestElapsed.Milliseconds}ms");
        playlistDetail = (JObject)playlistDetail["playlist"];
        Name = playlistDetail["name"].ToString();
        Description = playlistDetail["description"].ToString();
        Tags = ((JArray)playlistDetail["tags"]).Select(tag => tag.ToString()).ToArray();
        CoverUrl = playlistDetail["coverImgUrl"].ToString();
        CreateTime = DateTimeOffset.FromUnixTimeMilliseconds((long)playlistDetail["createTime"]).DateTime;
        Creator = playlistDetail["creator"]["nickname"].ToString();
        MusicTrackIds = ((JArray)playlistDetail["trackIds"]).Select(track => long.Parse(track["id"].ToString()))
            .ToArray();
    }

    public async Task InitMusicsAsync()
    {
        var tracks = (JArray)(await Apis.Music.Detail(MusicTrackIds, Nons.Instance))["songs"];
        Musics = new Music[tracks.Count];
        var (results, elapsed) = await Tools.MeasureExecutionTimeAsync(Task.WhenAll(
            tracks.Select(track => Task.Run(() => new Music((JObject)track))).ToList()));
        Array.Copy(results, Musics, results.Length);
        Debug.WriteLine($"实例化歌单({Id})每首歌曲所用时间: {elapsed.Milliseconds}ms");
    }

    public Stream GetPic(int x = 0, int y = 0)
    {
        var IMGSIZE = "?param=300y300";
        if (x == 0)
        {
            return HttpRequest.StreamHttpGet(CoverUrl + IMGSIZE);
        }

        return HttpRequest.StreamHttpGet(CoverUrl + $"?param={x}y{y}");
    }
}