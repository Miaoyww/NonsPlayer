using System.Text.RegularExpressions;

namespace NcmPlayer.Framework.Model
{
    public class Lrcs
    {
        public List<Lrc>? Lyrics;

        public int? Count => Lyrics.Count;

        public Lrcs(string lrc)
        {
            string[] sp = Regex.Split(lrc, @"\n");
            if (sp.Length != 0)
            {
                foreach (string item in sp)
                {
                    if (!item.Equals(""))
                    {
                        Lrc one = new(item);
                        Lyrics.Add(one);
                    }
                }
            }
            else
            {
                Lrc one = new(lrc);
                Lyrics.Add(one);
            }
        }
    }

    public class Lrc
    {
        public TimeSpan Showtime;
        public string LrcContent;

        public Lrc(string stringLrc)
        {
            string timeString = Regex.Match(stringLrc, "(?<=\\[)\\S*(?=])").ToString();
            string lrcString = Regex.Match(stringLrc, "(?<=(\\])).*").ToString();
            int minMs;
            int secMs;
            int ms;
            try
            {
                minMs = int.Parse(timeString.Split(":")[0]) * 60 * 1000;
                secMs = int.Parse(timeString.Split(":")[1].Split(".")[0]) * 1000;
                ms = int.Parse(timeString.Split(":")[1].Split(".")[1]);
            }
            catch (System.FormatException)
            {
                minMs = 0;
                secMs = 0;
                ms = 0;
            }
            Showtime = TimeSpan.FromMilliseconds(minMs + secMs + ms);
            try
            {
                if (lrcString[0].Equals(string.Empty) || lrcString[0].Equals(" "))
                {
                    lrcString = lrcString.Remove(0);
                }
            }
            catch (IndexOutOfRangeException)
            {
            }
            LrcContent = lrcString;
        }
    }
}