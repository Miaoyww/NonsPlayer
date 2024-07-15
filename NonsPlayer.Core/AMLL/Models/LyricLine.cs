namespace NonsPlayer.Core.AMLL.Models;

/// <summary>
/// 一行歌词
/// </summary>
public class LyricLine
{
    public TimeSpan StartTime { get; set; }
    public TimeSpan EndTime { get; set; }
    public List<LyricWord>? LyricWords { get; set; }
    public string? Pure { get; set; }
    public string? Translation { get; set; }

    public bool HaveTranslation => !string.IsNullOrEmpty(Translation);

    /// <summary>
    /// 是否为纯音乐
    /// </summary>
    public bool IsPureMusic { get; set; }

    /// <summary>
    /// 是否为逐词歌词
    /// </summary>
    public bool IsVerbatimLyrics => LyricWords?.Count == 0;

    public void SetTranslation(string translation)
    {
        Translation = translation;
    }

    public LyricLine(List<LyricWord>? spliteWords, string? translation = "",
        bool isPureMusic = false)
    {
        LyricWords = spliteWords;
        // 将逐词歌词合并为一行
        Pure = string.Join("", spliteWords.Select(x => x.Pure));
        Translation = translation;
        IsPureMusic = isPureMusic;
    }

    public LyricLine(string pureline, TimeSpan startTime, TimeSpan endTime, string? translation = "",
        bool isPureMusic = false)
    {
        Pure = pureline;
        Translation = translation;
        IsPureMusic = isPureMusic;

        StartTime = startTime;
        EndTime = endTime;
    }

    /// <summary>
    /// 空白歌词行
    /// </summary>
    public LyricLine(string pure)
    {
        Pure = pure;
        StartTime = TimeSpan.Zero;
        EndTime = TimeSpan.MaxValue;
    }

    public LyricLine()
    {
    }
}