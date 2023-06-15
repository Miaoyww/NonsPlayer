using System.Diagnostics;
using System.Text.RegularExpressions;
using Newtonsoft.Json.Linq;
using static System.Windows.Forms.VisualStyles.VisualStyleElement.TaskbarClock;

namespace NonsPlayer.Framework.Model
{
    public class Lyrics
    {
        public List<Lyric>? Lrc;
        public int? Count => Lrc.Count;

        public Lyrics(JObject lrc)
        {
            Lrc = new();
            var oArray = ParseLrc(lrc["lrc"]["lyric"].ToString());
            JArray tArray = new();
            if (lrc.Property("tlyric") != null)
            {
                tArray = ParseLrc(lrc["tlyric"]["lyric"].ToString());
            }
            // if (oArray == "")
            // {
            //     Lrc.Add(new Lyric("纯音乐 请欣赏", "", TimeSpan.Zero));
            //     return;
            // }
            foreach (var originalLrc in oArray)
            {
                if (tArray.Count > 0)
                {
                    foreach (var tranLrc in tArray)
                    {
                        if (!((TimeSpan)originalLrc["time"]).Equals((TimeSpan)tranLrc["time"]))
                        {
                            continue;
                        }

                        Lrc.Add(new Lyric(
                            originalLrc["word"].ToString(),
                            tranLrc["word"].ToString(),
                            (TimeSpan)originalLrc["time"]));
                    }
                }
                else
                {
                    Lrc.Add(new Lyric(
                        originalLrc["word"].ToString(),
                        "",
                        (TimeSpan)originalLrc["time"]));
                }
                
            }
        }


        private JArray ParseLrc(string content)
        {
            JArray value = new();
            var match = Regex.Matches(content, "\\[([0-9.:]*)\\]+(.*)", RegexOptions.Compiled);
            foreach (Match item in match)
            {
                value.Add(new JObject
                {
                    {"time", item.Groups[1].Value == "99:00.00" ? TimeSpan.MaxValue : TimeSpan.Parse("00:" + item.Groups[1].Value)},
                    {"word", item.Groups[2].Value}
                });
            }
            return value;
        }
    }

    public class Lyric
    {
        public TimeSpan Time;
        public string OriginalLyric;
        public string TranLyric;

        public Lyric(string oLrc, string tLyric, TimeSpan time)
        {
            OriginalLyric = oLrc;
            TranLyric = tLyric;
            Time = time;
        }
    }
}