using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;

namespace NonsPlayer.Core.Models;

public class LocalArtist: IArtist
{
    public string Id { get; set; }
    public string Md5 { get; set; }
    public string Name { get; set; }
    public string ShareUrl { get; set; }
    public string AvatarUrl { get; set; }
    public string Description { get; set; }
    public List<IMusic> HotMusics { get; set; }
    public int MusicCount { get; set; }
    public string Trans { get; set; }
    public IAdapter Adapter { get; set; }
}