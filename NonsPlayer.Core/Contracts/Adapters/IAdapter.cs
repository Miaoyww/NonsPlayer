namespace NonsPlayer.Core.Contracts.Adapters;

public class AdapterMetadata
{
    public AdapterMetadata This { get; set; }

    public AdapterMetadata()
    {
        This = this;
    }

    public string Name { get; set; }

    public string Platform { get; set; }

    /// <summary>
    /// 平台名
    /// 使用原名. 例如: 网易云音乐, QQ音乐等
    /// </summary>
    public string DisplayPlatform { get; set; }

    public string Author { get; set; }
    public string Description { get; set; }
    public Version Version { get; set; }
    public TimeSpan UpdateTime { get; set; }
    public Uri Repository { get; set; }

    public AdapterType Type { get; set; }
}

public enum AdapterType
{
    Common,
    OnlyMusic
}

public interface IAdapter
{
    IAccountAdapter Account { get; }
    ICommonAdapter Common { get; }
    ISearchAdapter Search { get; }
    IMusicAdapter Music { get; }
    IArtistAdapter Artist { get; }
    IAlbumAdapter Album { get; }
    IPlaylistAdapter Playlist { get; }
    AdapterMetadata GetMetadata();
}