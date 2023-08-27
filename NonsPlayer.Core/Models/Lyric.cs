using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Helpers;

namespace NonsPlayer.Core.Models;

public class LyricGroup
{
    [JsonPropertyName("lyrics")] public List<Lyric> Lyrics;

    public LyricGroup(JObject lrc)
    {
        Lyrics = new List<Lyric>();
        Tuple<TimeSpan, string>[] oArray = ParseLrc(lrc["lrc"]["lyric"].ToString());
        Tuple<TimeSpan, string>[]? tArray = null;
        Dictionary<TimeSpan, int> timeSpanTable = new();
        foreach (var item in lrc["lrc"]["lyric"].ToString().Split("\n"))
        {
            JObject newLyric;
            try
            {
                if (item.Contains("{"))
                {
                    newLyric = JObject.Parse(item);
                }
                else
                {
                    continue;
                }
            }
            catch (Exception e)
            {
                continue;
            }

            var time = TimeSpan.Zero;
            var word = string.Join("", newLyric["c"].Select(x => x["tx"].ToString()));
            Lyrics.Add(new Lyric(word, string.Empty, time));
        }

        if (lrc.Property("tlyric") != null)
        {
            tArray = ParseLrc(lrc["tlyric"]["lyric"].ToString());
            for (int i = 0; i < tArray.Length; i++)
            {
                var item = tArray[i];
                timeSpanTable[item.Item1] = i;
            }
        }
        
        foreach (var item in oArray)
        {
            var tLyric = string.Empty;
            if (tArray != null)
            {
                if (timeSpanTable.TryGetValue(item.Item1, out var index))
                {
                    tLyric = tArray[index].Item2;
                }
            }

            Lyrics.Add(new Lyric(item.Item2, tLyric, item.Item1));
        }
    }

    [JsonPropertyName("lyric_count")] public int? Count => Lyrics.Count;

    private Tuple<TimeSpan, string>[] ParseLrc(string content)
    {
        var match = Regex.Matches(content, "\\[([0-9.:]*)\\]+(.*)", RegexOptions.Compiled);
        var result = new Tuple<TimeSpan, string>[match.Count];
        for (var i = 0; i < match.Count; i++)
        {
            var item = match[i];
            TimeSpan time;
            if (item.Groups[1].Value == "99:00.00")
            {
                // 纯音乐或无歌词
                time = TimeSpan.MaxValue;
            }
            else
            {
                time = TimeSpan.Parse("00:" + item.Groups[1].Value);
            }

            var word = item.Groups[2].Value;
            result[i] = new Tuple<TimeSpan, string>(time, word);
        }

        return result;
    }
}

public class Lyric
{
    [JsonPropertyName("original_lyric")] public string OriginalLyric;

    [JsonPropertyName("time")] public TimeSpan Time;

    [JsonPropertyName("tran_lyric")] public string TranLyric;

    public Lyric(string oLrc, string tLyric, TimeSpan time)
    {
        OriginalLyric = oLrc;
        TranLyric = tLyric;
        Time = time;
    }
}