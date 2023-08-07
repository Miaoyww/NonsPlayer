using System.Diagnostics;

namespace NonsPlayer.Core.Helpers;

public static class Tools
{
    public static async Task<(TResult result, TimeSpan elapsed)> MeasureExecutionTimeAsync<TResult>(
        this Task<TResult> task)
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