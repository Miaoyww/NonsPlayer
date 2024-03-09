using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class MusicModel
{
    public string Index;
    public Music Music;
    public Tuple<string, string> Cover => Tuple.Create(Music.Album.CacheSmallAvatarId, Music.Album.SmallAvatarUrl);
    public string Name => Music.Name;
    public string Artists => Music.ArtistsName;
    public string Time => Music.TotalTimeString;

    public long Id => Music.Id;
}