using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAlbumAdapter: IAdapter
{
    Task<Album> GetAlbumAsync(object content);
    
    Album GetAlbum(object content);
}