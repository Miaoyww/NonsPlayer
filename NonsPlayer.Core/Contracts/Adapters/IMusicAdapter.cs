using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IMusicAdapter : ISubAdapter
{
    Task<IMusic?>? GetMusicAsync(object content);
    Task<IMusic?>? GetMusicAsyncById(string id);

    Task<IMusic[]?>? GetMusicListAsync(object content);
    Task<IMusic[]?>? GetMusicListByIdAsync(string[] ids);

    IMusic? GetMusic(object content);
    IMusic[]? GetMusicList(object content);
    
}