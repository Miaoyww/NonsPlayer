using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ICommonAdapter
{
    bool IsEnable { get; set; }
    
    Task<Playlist> GetHomePlaylistAsync(object content);
    
    Task<Music[]> GetMusicListAsync(object content);

    Music[] GetMusicList(object content);

    Music GetMusic(object content);
}