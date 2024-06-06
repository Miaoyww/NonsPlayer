using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IMusicAdapter : ISubAdapter
{
    Task<Music?>? GetMusicAsync(object content);

    Task<Music[]?>? GetMusicListAsync(object content);

    Music? GetMusic(object content);
    Music[]? GetMusicList(object content);
}