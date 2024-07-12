using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IMusicAdapter : ISubAdapter
{
    Task<IMusic?>? GetMusicAsync(object content);

    Task<IMusic[]?>? GetMusicListAsync(object content);

    IMusic? GetMusic(object content);
    IMusic[]? GetMusicList(object content);
    
    Task<string> GetMusicUrlAsync(string id, MusicQualityLevel qualityLevel);
}