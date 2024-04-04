using System.Diagnostics;
using System.Text.Json.Serialization;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Adapters;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Models;

public class Playlist : IPlaylist
{
    [JsonPropertyName("create_time")] public DateTime CreateTime;
    [JsonPropertyName("creator")] public string Creator;
    [JsonPropertyName("description")] public string Description;
    public bool IsCardMode;
    
    private MusicAdapters _musicAdapter = new MusicAdapters();

    [JsonPropertyName("music_track_ids")] public long[] MusicTrackIds;
    private JObject? PlaylistDetail;
    [JsonPropertyName("tags")] public string[] Tags;
    public string CacheId => Id + "_playlist";
    public Playlist This => this;

    public async Task LoadAsync(long id)
    {
        Id = id;
        var playlistDetail = await Apis.Playlist.Detail(Id, NonsCore.Instance);
        playlistDetail = (JObject)playlistDetail["playlist"];
        PlaylistDetail = playlistDetail;
        Name = playlistDetail["name"].ToString();
        Description = playlistDetail["description"].ToString();
        Tags = ((JArray)playlistDetail["tags"]).Select(tag => tag.ToString()).ToArray();
        AvatarUrl = playlistDetail["coverImgUrl"].ToString();
        CreateTime = DateTimeOffset.FromUnixTimeMilliseconds((long)playlistDetail["createTime"]).DateTime;
        Creator = playlistDetail["creator"]["nickname"].ToString();
        MusicTrackIds = ((JArray)playlistDetail["trackIds"]).Select(track => long.Parse(track["id"].ToString()))
            .ToArray();
    }

    /// <summary>
    ///     只会获取前1000个歌曲信息
    /// </summary>
    public void InitTracks()
    {
        // 直接从PlaylistDetail中获取歌曲信息, 因为它会传递小于等于1000的歌曲信息
        Musics =
        [
            ..((JArray)PlaylistDetail["tracks"])
            .Select(track => _musicAdapter.GetMusic((JObject)track))
            .ToList()
        ];
    }

    public async Task InitTrackByIndexAsync(int start, int end)
    {
        var ids = MusicTrackIds.Skip(start).Take(end - start).ToArray();
        var musicTrackIdsGroup = ids.Select((id, index) => new { id, index })
            .GroupBy(x => x.index / 200)
            .Select(x => x.Select(v => v.id).ToArray())
            .ToArray();
        var musicJObjectTasks = musicTrackIdsGroup.Select(x => Apis.Music.Detail(x, NonsCore.Instance));
        var (result, elapsed) = await Task.WhenAll(musicJObjectTasks).MeasureExecutionTimeAsync();
        Musics.AddRange(result.Select(item => _musicAdapter.GetMusicList(item["songs"] as JArray))
            .SelectMany(x => x));
        Debug.WriteLine($"获取歌曲({start}-{end})信息获取完毕 {elapsed.TotalMilliseconds}ms");
    }
}