using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;

namespace NonsPlayer.Core.Models;

public class LocalAlbum: IAlbum
{
    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
    public DateTime CreateDate { get; set; }
    public string Description { get; set; }
    public IMusic[] Musics { get; set; }
    public IArtist[] Artists { get; set; }
    public int CollectionCount { get; set; }
    public int TrackCount { get; set; }
    public IAdapter Adapter { get; set; }
}