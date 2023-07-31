using System.Diagnostics;
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

    private Playlist()
    {
    }

    public static async Task<Playlist> CreateAsync(long? id)
    {
        if (id == null)
        {
            throw new PlaylistIdNullException("歌单Id为空");
        }

        var playlist = new Playlist();
        await playlist.LoadAsync(id);
        return playlist;
    }

    private async Task LoadAsync(long? id)
    {
        Id = id;
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
        // 将MusicTrackIds分组，每组200个
        var musicTrackIdsGroup = MusicTrackIds.Select((id, index) => new {id, index})
            .GroupBy(x => x.index / 200)
            .Select(x => x.Select(v => v.id).ToArray())
            .ToArray();
        // 将每组的歌曲详情请求并异步解析
        var musicTasks = musicTrackIdsGroup.Select(musicTrackIds =>
            Apis.Music.Detail(musicTrackIds, Nons.Instance));
        var (result, elapsed) = await Task.WhenAll(musicTasks).MeasureExecutionTimeAsync();
        Debug.WriteLine($"歌曲Api请求所用时间: {elapsed.Milliseconds}ms");
        // 将每组的歌曲详情解析为Music对象
        Musics = result.SelectMany(x => x["songs"]).Select(x => Music.CreateAsync((JObject)x).Result).ToArray();
    }
}