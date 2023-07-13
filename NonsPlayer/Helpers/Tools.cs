using System.Diagnostics;
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

    public static async Task<(TResult result, TimeSpan elapsed)> MeasureExecutionTimeAsync<TResult>(this Task<TResult> task)
    {
        var stopwatch = Stopwatch.StartNew();
        var result = await task;
        stopwatch.Stop();
        return (result, stopwatch.Elapsed);
    }

    public static async Task<TimeSpan> MeasureExecutionTimeAsync(Task task)
    {
        var stopwatch = Stopwatch.StartNew();
        await task;
        stopwatch.Stop();
        return stopwatch.Elapsed;
    }
}