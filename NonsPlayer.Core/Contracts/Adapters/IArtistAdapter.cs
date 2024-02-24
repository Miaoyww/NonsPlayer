using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IArtistAdapter
{
    Task<Artist> GetArtistAsync(object content);
    
    Artist GetArtist(object content);
}