using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Models;

public class IMusic : INonsModel
{
    public Album Album;
    public Artist[] Artists;
    public bool IsEmpty;
    public TimeSpan Duration;
    public string Uri;
    public Lyric Lyric;
    public string AlbumName => Album?.Name;
    public string TotalTimeString => Duration.ToString(@"m\:ss");
    public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));
    
}