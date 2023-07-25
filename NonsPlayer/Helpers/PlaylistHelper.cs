using NonsPlayer.Contracts.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Helpers
{
    public static class PlaylistHelper
    {
        public static void OpenMusicListDetail(long id, INavigationService navigationService)
        {
            navigationService.NavigateTo(typeof(MusicListDetailViewModel).FullName!, id);
        }
    }
}