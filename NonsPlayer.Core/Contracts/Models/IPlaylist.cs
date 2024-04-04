using System.Text.Json.Serialization;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Models;

public class IPlaylist : INonsModel
{
    public List<IMusic> Musics;
    public int MusicsCount => Musics.Count;
    public bool IsLocal;
}