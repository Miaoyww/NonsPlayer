using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ICommonAdapter: ISubAdapter
{
    Task<Playlist[]> GetRecommendedPlaylistAsync(object content, int count, NonsCore core);
    
}