using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Adapters;
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
    [JsonPropertyName("musics")] public List<Music> Musics;

    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds;
    private JObject? PlaylistDetail;
    public string CacheId => Id + "_playlist";
    [JsonPropertyName("tags")] public string[] Tags;
    [JsonPropertyName("musics_count")] public int MusicsCount => MusicTrackIds.Length;
    public Playlist This => this;

    public async Task LoadAsync(long id)
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

    /// <summary>
    /// 只会获取前1000个歌曲信息
    /// </summary>
    public void InitTracksAsync()
    {
        // 直接从PlaylistDetail中获取歌曲信息, 因为它会传递小于等于1000的歌曲信息
        Musics = ((JArray) PlaylistDetail["tracks"])
            .Select(track => MusicAdapters.CreateFromPlaylistTrack((JObject) track))
            .ToList();
    }

    public async Task InitTrackByIndexAsync(int start, int end)
    {
        var ids = MusicTrackIds.Skip(start).Take(end - start).ToArray();
        var musicTrackIdsGroup = ids.Select((id, index) => new {id, index})
            .GroupBy(x => x.index / 200)
            .Select(x => x.Select(v => v.id).ToArray())
            .ToArray();
        var musicJObjectTasks = musicTrackIdsGroup.Select(x => Apis.Music.Detail(x, Nons.Instance));
        var (result, elapsed) = await Task.WhenAll(musicJObjectTasks).MeasureExecutionTimeAsync();
        Musics.AddRange(result.Select(item => MusicAdapters.CreateFromMuiscDetail(item["songs"] as JArray)).SelectMany(x => x));
        Debug.WriteLine($"获取歌曲({start}-{end})信息获取完毕 {elapsed.TotalMilliseconds}ms");
    }
}