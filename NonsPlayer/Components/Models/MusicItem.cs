using NonsPlayer.Core.Models;

namespace NonsPlayer.Components.Models;

public class MusicItem
{
    public Music Music;
    public string Index;
    public string Cover => Music.Album.SmallAvatarUrl;
    public string Name => Music.Name;
    public string Artists => Music.ArtistsName;
    public string Time => Music.TotalTimeString;

    public long Id => Music.Id;
}