using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IPlaylistAdapter
{
    Task<Playlist> GetPlaylistAsync(long id);

    Playlist GetPlaylist(long id);
}