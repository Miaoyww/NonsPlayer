using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using NcmPlayer.Contracts.Services;
using NcmPlayer.ViewModels;

namespace NcmPlayer.Helpers;

public static class Tools
{
    public static DateTime TimestampToDateTime(string timeStamp)
    {
        DateTime sTime = new DateTime(1970, 1, 1, 0, 0, 0).ToLocalTime();
        return sTime.AddSeconds(double.Parse(timeStamp));
    }

    public static void OpenMusicListDetail(long id, INavigationService navigationService)
    {
        navigationService.NavigateTo(typeof(MusicListDetailViewModel).FullName!, id);
    }
}