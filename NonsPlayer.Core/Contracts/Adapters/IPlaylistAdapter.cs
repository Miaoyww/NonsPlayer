using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IPlaylistAdapter : ISubAdapter
{
    Task<Playlist> GetPlaylistAsync(long id);
    List<Music> InitTracks(JObject pure);
}