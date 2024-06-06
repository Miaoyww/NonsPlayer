using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAlbumAdapter: ISubAdapter
{
    Task<Album> GetAlbumAsync(object content);
    
    Album GetAlbum(object content);
}