namespace NonsPlayer.Core.AMLL.Models;

/// <summary>
/// 用于逐字歌词的单词
/// </summary>
public class LyricWord
{
    public TimeSpan StartTime { get; set; }
    public TimeSpan EndTime { get; set; }
    public string Pure { get; set; }
}
