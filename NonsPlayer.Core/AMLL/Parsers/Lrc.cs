using System.Globalization;
using System.Text.RegularExpressions;
using NonsPlayer.Core.AMLL.Models;

namespace NonsPlayer.Core.AMLL.Parsers;

/// <summary>
/// Lrc文件解析器
/// </summary>
public static class Lrc
{
    public static Lyric ParseLrc(string lrcContent, TimeSpan musicEndTime)
    {
        var lines = lrcContent.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        //去除以{开头的行
        lines = lines.Where(x => !x.StartsWith("{")).ToArray();
        var lyricLines = new List<LyricLine>();
        TimeSpan? lastTime = null;

        for (int i = 0; i < lines.Length; i++)
        {
            var line = lines[i];
            TimeSpan endTime;
            if (i + 1 >= lines.Length)
            {
                endTime = musicEndTime;
            }
            else
            {
                var nextLine = lines[i + 1];
                TryParseTime(nextLine, out TimeSpan time);
                endTime = time;
            }

            TryParseTime(line, out TimeSpan startTime);
            lastTime = endTime - startTime;
            var lyricText = line.Substring(line.LastIndexOf(']') + 1).Trim();
            if (!string.IsNullOrWhiteSpace(lyricText))
            {
                var lyricLine = new LyricLine(lyricText, startTime, endTime, isPureMusic: false);
                lyricLines.Add(lyricLine);
            }
        }
        return new Lyric(lyricLines);
    }

    private static bool TryParseTime(string line, out TimeSpan time)
    {
        time = TimeSpan.Zero;
        var matches = Regex.Matches(line, @"\[(\d+):(\d+\.\d+)]");
        if (matches.Count > 0)
        {
            var match = matches[0];
            if (match.Groups.Count == 3)
            {
                var minutes = int.Parse(match.Groups[1].Value);
                var seconds = double.Parse(match.Groups[2].Value, CultureInfo.InvariantCulture);
                time = TimeSpan.FromMinutes(minutes) + TimeSpan.FromSeconds(seconds);
            }

            return true;
        }

        return false;
    }
}