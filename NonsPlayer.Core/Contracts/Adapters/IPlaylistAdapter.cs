using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IPlaylistAdapter : IAdapter
{
    Task<Playlist> GetPlaylistAsync(long id);
}