using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IMusicAdapter: IAdapter
{
    Task<Music?>? GetMusicAsync(object content);
    
    Task<Music[]?>? GetMusicListAsync(object content);
}