using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IArtistAdapter: ISubAdapter
{
    //TODO 
    Task<IArtist> GetArtistAsyncById(string id);

    IArtist GetArtist(object content);
}