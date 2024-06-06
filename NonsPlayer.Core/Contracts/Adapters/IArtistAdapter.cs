using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IArtistAdapter: ISubAdapter
{
    Task<Artist> GetArtistAsync(object content);
    
    Artist GetArtist(object content);
}