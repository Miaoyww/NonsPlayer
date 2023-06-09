namespace NonsPlayer.Framework.Model
{
    /*
    public static class HttpRequest
    {
        public static async Task<Stream> StreamHttpGet(string url, int start = 0, int end = 0)
        {
            WebRequest wrGETURL;
            wrGETURL = WebRequest.Create(url);
            if (end != 0)
            {
                wrGETURL.Headers.Add("Range", $"bytes={start}-{end}");
            }
            Stream objStream;
            while (true)
            {
                try
                {
                    objStream = wrGETURL.GetResponse().GetResponseStream();
                    StreamReader objReader = new StreamReader(objStream);
                    break;
                }
                catch (WebException)
                {
                    Thread.Sleep(3);
                }
                catch (InvalidOperationException)
                {
                    return null;
                }
            }

            return objStream;
        }

        public static async Task<JObject> JObjectHttpGet(Stream stream)
        {
            StreamReader objReader = new StreamReader(stream);
            return (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());
        }

        public static Task<JObject> GetJson(string url)
        {
            return JObjectHttpGet(StreamHttpGet(url).Result);
        }
    }

    public static class Tool
    {
        public static DateTime TimestampToDateTime(string timeStamp)
        {
            DateTime sTime = new DateTime(1970, 1, 1, 0, 0, 0).ToLocalTime();
            return sTime.AddSeconds(double.Parse(timeStamp));
        }

        public static List<Page> OpenedPlaylistDetail = new List<Page>();

        public static async void OpenPlayListDetail(int id)
        {
            Stopwatch stopwatch = new();
            Playlist newone = new()
            {
                Title = id.ToString()
            };
            PublicMethod.ChangePage(newone);
            Task get = new Task(async () =>
            {
                PlayList playList = new(id);
                await newone.Dispatcher.BeginInvoke(new Action(() =>
                {
                    string name = playList.Name;
                    string creator = playList.Creator;
                    string description = playList.Description;
                    string createTime = playList.CreateTime.ToString();
                    int musicsCount = playList.MusicsCount;
                    newone.Name = name;
                    newone.Creator = creator;
                    newone.CreateTime = createTime;
                    newone.Description = description;
                    newone.MusicsCount = musicsCount.ToString();
                }));
                Thread getCover = new(async _ =>
                {
                    stopwatch.Restart();
                    Stream playlistCover = HttpRequest.StreamHttpGet(playList.PicUrl).Result;
                    await newone.Dispatcher.BeginInvoke(new Action(() =>
                    {
                        newone.SetCover(playlistCover);
                    }));
                    stopwatch.Stop();
                    Debug.WriteLine($"OpenPlayListDetail 获取封面耗时{stopwatch.ElapsedMilliseconds}");
                });
                getCover.IsBackground = true;
                getCover.Start();
                stopwatch.Restart();
                Music[] musics = await playList.InitArtWorkList();

                await newone.Dispatcher.BeginInvoke(new Action(async () =>
                {
                    await newone.UpdateMusicsList(musics, playList);
                }));
                stopwatch.Stop();
                Debug.WriteLine($"OpenPlayListDetail 更新歌曲耗时{stopwatch.ElapsedMilliseconds}");
            });
            get.Start();
        }
    }
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
    }*/
}