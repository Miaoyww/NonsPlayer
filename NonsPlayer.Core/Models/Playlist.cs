using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Exceptions;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class Playlist : INonsModel
{
    [JsonPropertyName("create_time")] public DateTime CreateTime;
    [JsonPropertyName("creator")] public string Creator;
    [JsonPropertyName("description")] public string Description;
    public bool IsCardMode;
    public string CacheId => Id + "_playlist";
    [JsonPropertyName("musics")] public Music[] Musics;

    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds;

    private JObject? PlaylistDetail;

    [JsonPropertyName("tags")] public string[] Tags;

    private Playlist()
    {
    }

    [JsonPropertyName("musics_count")] public int MusicsCount => MusicTrackIds.Length;
    public Playlist This => this;

    public static async Task<Playlist> CreateCardModeAsync(JObject? item)
    {
        if (item == null) throw new PlaylistIdNullException("歌单JObject为空");

        var playlist = new Playlist
        {
            Id = (long) item["id"],
            Name = item["name"].ToString(),
            AvatarUrl = item["picUrl"].ToString(),
            IsCardMode = true
        };
        return playlist;
    }

    public static async Task<Playlist> CreateAsync(long? id)
    {
        if (id == null) throw new PlaylistIdNullException("歌单Id为空");

        var playlist = new Playlist();
        await playlist.LoadAsync(id).ConfigureAwait(false);
        return playlist;
    }

    public static async Task<Playlist> CreateAsync(JObject item)
    {
        var playlist = new Playlist
        {
            Id = (long) item["id"],
            Name = item["name"].ToString(),
            AvatarUrl = item["picUrl"].ToString(),
        };
        await playlist.LoadAsync(playlist.Id).ConfigureAwait(false);
        return playlist;
    }

    public async Task LoadAsync(long? id)
    {
        Id = id;
        var playlistDetail = await Apis.Playlist.Detail(Id, Nons.Instance);
        playlistDetail = (JObject) playlistDetail["playlist"];
        PlaylistDetail = playlistDetail;
        Name = playlistDetail["name"].ToString();
        Description = playlistDetail["description"].ToString();
        Tags = ((JArray) playlistDetail["tags"]).Select(tag => tag.ToString()).ToArray();
        AvatarUrl = playlistDetail["coverImgUrl"].ToString();
        CreateTime = DateTimeOffset.FromUnixTimeMilliseconds((long) playlistDetail["createTime"]).DateTime;
        Creator = playlistDetail["creator"]["nickname"].ToString();
        MusicTrackIds = ((JArray) playlistDetail["trackIds"]).Select(track => long.Parse(track["id"].ToString()))
            .ToArray();
    }

    public async Task InitMusicsAsync()
    {
        // 直接从PlaylistDetail中获取歌曲信息, 因为它会传递小于等于1000的歌曲信息
        var musicTasks = ((JArray) PlaylistDetail["tracks"]).Select(track => Music.CreateAsync((JObject) track))
            .ToArray();
        if (MusicsCount >= 1000)
        {
            Debug.WriteLine("当前歌单歌曲数量大于等于1000, 正在获取剩余歌曲信息");
            // 删除前1000个歌曲, 因为它们已经被获取过了
            MusicTrackIds = MusicTrackIds.Skip(1000).ToArray();
            var musicTrackIdsGroup = MusicTrackIds.Select((id, index) => new {id, index})
                .GroupBy(x => x.index / 200)
                .Select(x => x.Select(v => v.id).ToArray())
                .ToArray();
            // musicTasks向后添加
            var musicJObjectTasks = musicTrackIdsGroup.Select(x => Apis.Music.Detail(x, Nons.Instance));
            var (joBjectResult, elapsed) = await Task.WhenAll(musicJObjectTasks).MeasureExecutionTimeAsync();
            Debug.WriteLine($"剩余歌曲信息获取完毕({elapsed.TotalMilliseconds}ms)");
            // 解析JObject并合并到musicTasks中
            var musicTasks2 = joBjectResult
                .SelectMany(x => ((JArray) x["songs"]).Select(track => Music.CreateAsync((JObject) track)))
                .ToArray();
            musicTasks = musicTasks.Concat(musicTasks2).ToArray();
        }

        var (result, elapsed2) = await Task.WhenAll(musicTasks).MeasureExecutionTimeAsync();
        Musics = result;
    }
}