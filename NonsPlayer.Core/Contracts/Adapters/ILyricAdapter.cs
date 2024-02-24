using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ILyricAdapter
{
    Task<Lyric> GetLyricAsync(object content);

    Lyric GetLyric(object content);
}