using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IArtistAdapter: ISubAdapter
{
    Task<Artist> GetArtistAsyncById(long id);
    
    Artist GetArtist(object content);
}