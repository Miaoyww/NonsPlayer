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

    string Key { get; set; }
    Task<Uri> GetQrCodeUrlAsync();
    Task<QrCodeResult> CheckLoginAsync(string key);

    Task<IAccount> GetAccountAsync(string token);

    Task<string> GetTokenAsync(string response);

    Task GetUserPlaylists();
    Task GetFavoritePlaylist();
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