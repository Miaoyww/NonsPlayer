using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Models;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ILyricAdapter: ISubAdapter
{
    Task<Lyric> GetLyricAsync(string id);
    
    Lyric GetLyric(string id);
}