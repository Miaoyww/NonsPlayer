using System.Text.RegularExpressions;
using NonsPlayer.Core.AMLL.Models;

namespace NonsPlayer.Core.AMLL.Parsers;

public class Yrc
{
    private static (int, int) ProcessTime(string startTime, string duration)
    {
        return (int.Parse(startTime), int.Parse(duration));
    }

    private static (string, (int, int)) ParseTime(string src)
    {
        var match = Regex.Match(src, @"\[(\d+),(\d+)\]");
        if (!match.Success) throw new Exception("Invalid time format");

        var times = ProcessTime(match.Groups[1].Value, match.Groups[2].Value);
        var remaining = src.Substring(match.Index + match.Length);
        return (remaining, times);
    }

    private static (string, (int, int)) ParseWordTime(string src)
    {
        var match = Regex.Match(src, @"\((\d+),(\d+),0\)");
        if (!match.Success) throw new Exception("Invalid word time format");

        var times = ProcessTime(match.Groups[1].Value, match.Groups[2].Value);
        var remaining = src.Substring(match.Index + match.Length);
        return (remaining, times);
    }

    private static (string, List<LyricWord>) ParseWords(string src)
    {
        var words = new List<LyricWord>();
        while (src.Length > 0)
        {
            var timeResult = ParseWordTime(src);
            src = timeResult.Item1;
            var wordEndIndex = src.IndexOf('(') != -1 ? src.IndexOf('(') : src.Length;
            var word = src.Substring(0, wordEndIndex);
            src = src.Substring(wordEndIndex);

            words.Add(new LyricWord
            {
                StartTime = TimeSpan.FromMilliseconds(timeResult.Item2.Item1),
                EndTime = TimeSpan.FromMilliseconds(timeResult.Item2.Item1 + timeResult.Item2.Item2),
                Pure = word
            });
        }

        return (src, words);
    }

    private static LyricLine ParseLine(string src)
    {
        var timeResult = ParseTime(src);
        src = timeResult.Item1;
        var wordsResult = ParseWords(src);
        src = wordsResult.Item1;

        return new LyricLine(wordsResult.Item2);
    }

    public static Lyric ParseYrc(string src)
    {
        var lines = src.Split(new[] { "\r\n", "\n" }, StringSplitOptions.None);
        var result = new List<LyricLine>();
        // 去除 作词 作曲 等无用信息
        foreach (var line in lines)
        {
            if (!string.IsNullOrWhiteSpace(line))
            {
                if (!line.StartsWith("["))
                {
                    continue;
                }
                result.Add(ParseLine(line));
            }
        }

        return new Lyric(result);
    }

    public static string StringifyYrc(List<LyricLine> lines)
    {
        var result = "";

        foreach (var line in lines)
        {
            if (line.LyricWords.Count > 0)
            {
                var startTime = line.LyricWords.First().StartTime;
                var duration = line.LyricWords.Sum(w => w.EndTime.TotalMilliseconds - w.StartTime.TotalMilliseconds);
                result += $"[{startTime},{duration}]";
                foreach (var word in line.LyricWords)
                {
                    var wordDuration = word.EndTime - word.StartTime;
                    result += $"({word.StartTime},{wordDuration},0){word.Pure}";
                }

                result += "\n";
            }
        }

        return result.Replace("(", "（").Replace(")", "）"); // 替换为中文括号
    }
}