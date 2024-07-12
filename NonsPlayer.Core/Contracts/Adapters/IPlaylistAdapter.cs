using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IPlaylistAdapter : ISubAdapter
{
    Task<IPlaylist> GetPlaylistAsync(long id);
    List<IMusic> InitTracks(JObject pure);
}