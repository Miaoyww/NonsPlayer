using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class MusicModel
{
    public string Index;
    public IMusic Music;

    public Tuple<string, string, byte[]> Cover =>
        Tuple.Create(Music.Album.CacheSmallAvatarId, Music.Album.SmallAvatarUrl, Music.LocalCover);

    public string Name => Music.Name;
    public string Artists => Music.ArtistsName;
    public string Time => Music.TotalTimeString;

    public long Id => Music.Id;
}