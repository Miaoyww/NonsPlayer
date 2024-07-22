using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;

namespace NonsPlayer.Core.Contracts.Adapters;

public interface ICommonAdapter: ISubAdapter
{
    Task<IPlaylist[]> GetRecommendedPlaylistAsync(int count);
    Task<IMusic[]> GetDailyRecommended();
    Task<IMusic[]> GetRadioSong();
}