using NonsPlayer.Contracts.Services;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Helpers;

public static class Tools
{
    public static DateTime TimestampToDateTime(string timeStamp)
    {
        var sTime = new DateTime(1970, 1, 1, 0, 0, 0).ToLocalTime();
        return sTime.AddSeconds(double.Parse(timeStamp));
    }

    public static void OpenMusicListDetail(long id, INavigationService navigationService)
    {
        navigationService.NavigateTo(typeof(MusicListDetailViewModel).FullName!, id);
    }
}