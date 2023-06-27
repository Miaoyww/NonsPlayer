using Newtonsoft.Json.Linq;
using NonsApi;
using NonsPlayer.Helpers;

namespace NonsPlayer.Framework.Model;

public class PlayList
{
    /// <summary>
    ///     歌单创建时间
    /// </summary>
    public DateTime CreateTime;

    /// <summary>
    ///     歌单歌曲全部Id
    /// </summary>
    public long[] MusicTrackIds;

    /// <summary>
    ///     歌单标签
    /// </summary>
    public string[] Tags;

    /// <summary>
    ///     歌单名称
    /// </summary>
    public string Name
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单Id
    /// </summary>
    public long Id
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单封面Url
    /// </summary>
    public string PicUrl
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单描述
    /// </summary>
    public string Description
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单创建者
    /// </summary>
    public string Creator
    {
        get;
        set;
    }

    /// <summary>
    ///     歌单歌曲数量
    /// </summary>
    public int MusicsCount => MusicTrackIds.Length;

    /// <summary>
    ///     通过Id打开歌单
    /// </summary>
    public async Task LoadAsync(long in_id)
    {
        Id = in_id;
        var playlistDetail = (JObject)(await NonsApi.Api.Playlist.Detail(Id, Nons.Instance))["playlist"];
        Name = playlistDetail["name"].ToString();
        Description = playlistDetail["description"].ToString();

        var jsonTags = (JArray)playlistDetail["tags"];
        Tags = new string[jsonTags.Count];
        for (var index = 0; index < Tags.Length; index++)
        {
            Tags[index] = jsonTags[index].ToString();
        }

        PicUrl = playlistDetail["coverImgUrl"].ToString();
        var jsonMusics = (JArray)playlistDetail["trackIds"];
        MusicTrackIds = new long[jsonMusics.Count];
        for (var index = 0; index < MusicTrackIds.Length; index++)
        {
            MusicTrackIds[index] = (int)jsonMusics[index]["id"];
        }

        var timestampTemp = playlistDetail["createTime"].ToString();
        CreateTime = Tools.TimestampToDateTime(timestampTemp.Remove(timestampTemp.Length - 3));
        Creator = playlistDetail["creator"]["nickname"].ToString();
    }

    public async Task<Music[]> InitArtWorkList(int start = 0, int end = 0)
    {
        JArray musicDetail;
        if (end != 0)
        {
            musicDetail = (JArray)(await NonsApi.Api.Music.Detail(MusicTrackIds[start..end], Nons.Instance))["musics"];
        }
        else
        {
            if (MusicTrackIds.Length >= 500)
            {
                var temp = await NonsApi.Api.Music.Detail(MusicTrackIds[..500], Nons.Instance);
                musicDetail = (JArray)temp["songs"];
            }
            else
            {
                var temp = await NonsApi.Api.Music.Detail(MusicTrackIds, Nons.Instance);
                musicDetail = (JArray)temp["songs"];
            }
        }

        var musics = new Music[musicDetail.Count];
        for (var index = 0; index < musics.Length; index++)
        {
            musics[index] = new Music((JObject)musicDetail[index]);
        }

        return musics;
    }

    public Stream GetPic(int x = 0, int y = 0)
    {
        var IMGSIZE = "?param=300y300";
        if (x == 0)
        {
            return HttpRequest.StreamHttpGet(PicUrl + IMGSIZE);
        }

        return HttpRequest.StreamHttpGet(PicUrl + $"?param={x}y{y}");
    }
}