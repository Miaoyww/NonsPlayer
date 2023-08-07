using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using Newtonsoft.Json.Linq;

namespace NonsPlayer.Core.Models;

public class Lyrics
{
    [JsonPropertyName("lyrics")] public List<Lyric>? Lrc;

    public Lyrics(JObject lrc)
    {
        Lrc = new List<Lyric>();
        var oArray = ParseLrc(lrc["lrc"]["lyric"].ToString());
        JArray tArray = new();
        if (lrc.Property("tlyric") != null) tArray = ParseLrc(lrc["tlyric"]["lyric"].ToString());
        // TODO: 翻译与原句个数不符的歌曲不显示歌词 Every Second - Mina Okabe
        foreach (var originalLrc in oArray)
            if (tArray.Count > 0)
                foreach (var tranLrc in tArray)
                {
                    if (!((TimeSpan) originalLrc["time"]).Equals((TimeSpan) tranLrc["time"])) continue;

                    Lrc.Add(new Lyric(
                        originalLrc["word"].ToString(),
                        tranLrc["word"].ToString(),
                        (TimeSpan) originalLrc["time"]));
                }
            else
                Lrc.Add(new Lyric(
                    originalLrc["word"].ToString(),
                    "",
                    (TimeSpan) originalLrc["time"]));
    }

    [JsonPropertyName("lyric_count")] public int? Count => Lrc.Count;


    private JArray ParseLrc(string content)
    {
        JArray value = new();
        var match = Regex.Matches(content, "\\[([0-9.:]*)\\]+(.*)", RegexOptions.Compiled);
        foreach (Match item in match)
            value.Add(new JObject
            {
                {
                    "time",
                    item.Groups[1].Value == "99:00.00"
                        ? TimeSpan.MaxValue
                        : TimeSpan.Parse(item.Groups[1].Value)
                },
                {"word", item.Groups[2].Value}
            });
        return value;
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