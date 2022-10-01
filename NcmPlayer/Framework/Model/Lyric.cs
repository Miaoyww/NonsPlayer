using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;

namespace NcmPlayer.Framework.Model
{
    public class Lrcs
    {
        private List<Lrc> lrcs = new List<Lrc>();

        public int Count
        {
            get => lrcs.Count;
        }

        public List<Lrc> GetLrcs
        {
            get => lrcs;
        }

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
                        lrcs.Add(one);
                    }
                }
            }
            else
            {
                Lrc one = new(lrc);
                lrcs.Add(one);
            }
        }
    }

    public class Lrc
    {
        private TimeSpan time;
        private string lrc;

        public TimeSpan GetTime
        {
            get => time;
        }

        public string GetLrc
        {
            get => lrc;
        }

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
            time = TimeSpan.FromMilliseconds(minMs + secMs + ms);
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
            lrc = lrcString;
        }
    }
}
