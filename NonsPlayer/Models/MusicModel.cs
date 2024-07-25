using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class MusicModel
{
    public string Index;
    public IMusic Music;

    public Tuple<string, string, byte[]> Cover =>
        Tuple.Create(Music.Album.CacheSmallAvatarId, Music.Album.SmallAvatarUrl, Music.LocalCover);
}