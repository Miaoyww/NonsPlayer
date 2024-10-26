using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Models;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface IAccountAdapter : ISubAdapter
{
    IPlaylist FavoritePlaylist { get; set; }
    List<IPlaylist> CreatedPlaylists { get; set; }
    List<IPlaylist> SavedPlaylists { get; set; }
    List<string> LikedSongs { get; set; }

    Task<Tuple<Uri, string>> GetQrCodeUrlAsync();
    Task<QrCodeResult> CheckLoginAsync(string key);
    IAccount GetAccount();
    Task<string> GetTokenAsync(string response);
    Task<bool> GetUserPlaylists();
    Task<bool> GetFavoritePlaylist();

    /// <summary>
    /// 获取是否收藏歌曲
    /// </summary>
    /// <param name="id"></param>
    /// <returns>收藏状态</returns>
    Task<bool> IsLikedSong(string id);

    Task UpdateLikeList();
}

public class QrCodeResult
{
    public string? Key { get; set; }
    public QrCodeStatus? Status { get; set; }
    public IAccount? Account { get; set; }
    public Uri? QrCodeUrl { get; set; }
}

public enum QrCodeStatus
{
    Waiting,
    Scanned,
    Confirmed,
    Timeout,
    Cancelled
}