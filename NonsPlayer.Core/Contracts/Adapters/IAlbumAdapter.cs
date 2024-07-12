using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAlbumAdapter: ISubAdapter
{
    Task<IAlbum> GetAlbumAsync(object content);
    
    IAlbum GetAlbum(object content);
}