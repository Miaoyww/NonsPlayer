using NcmApi;
using NcmPlayer.Resources;
using NcmPlayer.Views;
using NcmPlayer.Views.Pages;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Text.RegularExpressions;
using System.Threading;

namespace NcmPlayer.CloudMusic
{
    public static class HttpRequest
    {
        public static Stream StreamHttpGet(string url, int start = 0, int end = 0)
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
                    return objStream;
                }
                catch (WebException)
                {
                    Thread.Sleep(3);
                }
                catch (InvalidOperationException)
                {
                }
            }
        }

        public static JObject JObjectHttpGet(Stream stream)
        {
            StreamReader objReader = new StreamReader(stream);
            return (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());
        }

        public static JObject GetJson(string url)
        {
            return JObjectHttpGet(StreamHttpGet(url));
        }
    }

    public static class Tool
    {
        public static DateTime TimestampToDateTime(string timeStamp)
        {
            DateTime sTime = new DateTime(1970, 1, 1, 0, 0, 0).ToLocalTime();
            return sTime.AddSeconds(double.Parse(timeStamp));
        }

        public static void OpenPlayListDetail(string id)
        {
            ThreadPool.QueueUserWorkItem(_ =>
            {
                MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
                {
                    Playlist newone = new();
                    PublicMethod.ChangePage(newone);
                    ThreadPool.QueueUserWorkItem(_ =>
                    {
                        MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
                        {
                            PlayList playList = new(id);
                            string name = playList.Name;
                            string creator = playList.Creator;
                            string description = playList.Description;
                            string createTime = playList.CreateTime.ToString();
                            int songsCount = playList.SongsCount;
                            newone.Name = name;
                            newone.Creator = creator;
                            newone.CreateTime = createTime;
                            newone.Description = description;
                            newone.SongsCount = songsCount.ToString();
                            ThreadPool.QueueUserWorkItem(_ =>
                            {
                                newone.Dispatcher.BeginInvoke(new Action(() =>
                                {
                                    Stream playlistCover = playList.Cover;
                                    newone.SetCover(playlistCover);
                                }));
                            });
                            Song[] songs = playList.InitArtWorkList();
                            newone.UpdateSongsList(songs);
                        }));
                    });
                }));
            });
        }
    }

    public class CloudMusic
    {
        private string id = string.Empty;
        private string name = string.Empty;
        private Stream? cover;
        private string coverUrl;
        private readonly string IMGSIZE = "?param=400y400";

        public string Id
        {
            get
            {
                return id;
            }
            set
            {
                id = value;
            }
        }

        public string Name
        {
            get
            {
                return name;
            }
            set
            {
                name = value;
            }
        }

        public Stream Cover
        {
            get
            {
                cover = HttpRequest.StreamHttpGet(coverUrl + IMGSIZE);
                return cover;
            }
            set
            {
                cover = value;
            }
        }

        public string CoverUrl
        {
            get
            {
                return coverUrl;
            }
            set
            {
                coverUrl = value;
            }
        }
    }

    public class PlayList : CloudMusic
    {
        private string description = string.Empty;
        private string[] tags;
        private string[] songsId;
        private string[] songTrackIds;
        private DateTime createTime;
        private string creator = String.Empty;
        private Song[] songs;
        private bool[] threadDone;
        private List<JArray> songPages = new List<JArray>();

        public PlayList(string in_id)
        {
            Id = in_id;
            Stopwatch stopwatch = new();
            stopwatch.Start();
            JObject playlistDetail = (JObject)Api.Playlist.Detail(Id, ResEntry.ncm)["playlist"];
            stopwatch.Stop();
            Debug.WriteLine($"获取歌单详情耗时{stopwatch.ElapsedMilliseconds}ms");
            Name = playlistDetail["name"].ToString();
            description = playlistDetail["description"].ToString();

            JArray jsonTags = (JArray)playlistDetail["tags"];
            tags = new string[jsonTags.Count];
            for (int index = 0; index < tags.Length; index++)
            {
                tags[index] = jsonTags[index].ToString();
            }

            CoverUrl = playlistDetail["coverImgUrl"].ToString();
            JArray jsonSongs = (JArray)playlistDetail["trackIds"];
            songTrackIds = new string[jsonSongs.Count];
            for (int index = 0; index < songTrackIds.Length; index++)
            {
                songTrackIds[index] = jsonSongs[index]["id"].ToString();
            }
            string timestampTemp = playlistDetail["createTime"].ToString();
            createTime = Tool.TimestampToDateTime(timestampTemp.Remove(timestampTemp.Length - 3));
            creator = playlistDetail["creator"]["nickname"].ToString();
        }

        public Song[] InitArtWorkList()
        {
            JArray songDetail;
            if (songTrackIds.Length >= 500)
            {
                songDetail = (JArray)Api.Song.Detail(songTrackIds[0..500], ResEntry.ncm)["songs"];
            }
            else
            {
                songDetail = (JArray)Api.Song.Detail(songTrackIds, ResEntry.ncm)["songs"];
            }
            songs = new Song[songDetail.Count];
            for (int index = 0; index < songs.Length; index++)
            {
                songs[index] = new Song((JObject)songDetail[index]);
            }
            return songs;
        }

        private void GetSongDetail(object parm)
        {
            object[] data = parm as object[];
            Stopwatch stopwatch = new();
            stopwatch.Start();
            Song song = new(data[0].ToString());
            songs[(int)data[1]] = song;
            stopwatch.Stop();
            Debug.WriteLine($"创建id为{data[0]}的歌曲花费了{stopwatch.ElapsedMilliseconds}ms");
            threadDone[(int)data[1]] = true;
        }

        public string[] SongsId
        {
            get
            {
                return songsId;
            }
        }

        public string Creator
        {
            get => creator;
        }

        public string Description
        {
            get => description;
        }

        public int SongsCount
        {
            get => songTrackIds.Length;
        }

        public DateTime CreateTime
        {
            get => createTime;
        }
    }

    public class Song : CloudMusic
    {
        private string songUrl = string.Empty;
        private string songType = string.Empty;
        private string[] artists;
        private string albumName = string.Empty;
        private string albumId = string.Empty;
        private TimeSpan duartionTime = TimeSpan.Zero;
        private string duartionTimeString = string.Empty;
        private string lrcString = string.Empty;
        private int songSize = 0;
        private List<int[]> sizeRange = new List<int[]>();
        private int chuckIndex = 0;
        private Lrcs lrc;

        // 哈，注意了，这里的JObject是由Playlist.Tracks获得的
        public Song(JObject playlistSongTrack)
        {
            Name = playlistSongTrack["name"].ToString();
            Id = playlistSongTrack["id"].ToString();
            duartionTime = TimeSpan.FromMilliseconds(int.Parse(playlistSongTrack["dt"].ToString()));
            duartionTimeString = duartionTime.ToString(@"m\:ss");
            CoverUrl = playlistSongTrack["al"]["picUrl"].ToString();
            albumName = playlistSongTrack["al"]["name"].ToString();
            albumId = playlistSongTrack["al"]["id"].ToString();
            JArray artistsJson = (JArray)playlistSongTrack["ar"];
            artists = new string[artistsJson.Count];
            for (int index = 0; index < artists.Length; index++)
            {
                artists[index] = artistsJson[index]["name"].ToString();
            }
        }
        public Song(string in_id)
        {
            Id = in_id;
            JObject songDetail;
            try
            {
                songDetail = (JObject)((JArray)Api.Song.Detail(new string[] { Id }, ResEntry.ncm)["songs"])[0];
            }
            catch (InvalidCastException)
            {
                throw new InvalidCastException($"未能发现此音乐{in_id}");
            }
            Name = songDetail["name"].ToString();
            JArray artistsJson = (JArray)songDetail["ar"];
            artists = new string[artistsJson.Count];
            for (int index = 0; index < artists.Length; index++)
            {
                artists[index] = artistsJson[index]["name"].ToString();
            }

            duartionTime = TimeSpan.FromMilliseconds(int.Parse(songDetail["dt"].ToString()));
            duartionTimeString = duartionTime.ToString(@"m\:ss");
            albumId = songDetail["al"]["id"].ToString();
            albumName = songDetail["al"]["name"].ToString();
            CoverUrl = songDetail["al"]["picUrl"].ToString();
        }

        public Stream GetWaveStream()
        {
            string _songUrl = SongUrl;
            // int[] range = sizeRange[chuckIndex];
            // Stream songStream = HttpRequest.StreamHttpGet(_songUrl, range[0], range[1]);
            Stream songStream = HttpRequest.StreamHttpGet(_songUrl);
            // chuckIndex++;
            return songStream;
        }
        public string GetMp3()
        {
            string _songUrl = SongUrl;
            string path = AppConfig.SongsPath(Id, SongType);
            if (!File.Exists(path))
            {
                if (!Directory.Exists(AppConfig.SongsDirectory))
                {
                    Directory.CreateDirectory(AppConfig.SongsDirectory);
                }
                FileStream fs = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
                Stream songStream = HttpRequest.StreamHttpGet(_songUrl);
                byte[] bArr = new byte[1024];
                int size = songStream.Read(bArr, 0, bArr.Length);
                while (size > 0)
                {
                    //stream.Write(bArr, 0, size);
                    fs.Write(bArr, 0, size);
                    size = songStream.Read(bArr, 0, bArr.Length);
                }
                fs.Close();
                songStream.Close();
            }
            return path;
        }
        private void getSongFileInfo()
        {
            JObject temp = (JObject)Api.Song.Url(new string[] { Id }, ResEntry.ncm)["data"][0];
            sizeRange.Clear();
            songSize = (int)temp["size"];
            songUrl = temp["url"].ToString();
            songType = temp["type"].ToString();

            int step = songSize / 8; // 这里是一块的大小，分8块分段请求
            int difference = songSize - (step * 8);
            int lastChuckSize = 0;
            for (int i = 1; i < step - 1; i++)
            {
                int currentChuckSize = (step * i);
                int[] chuck;
                if (songSize - currentChuckSize == difference)
                {
                    chuck = new int[] { lastChuckSize, songSize};
                }
                else if(currentChuckSize < songSize)
                {
                    chuck = new int[] { lastChuckSize, currentChuckSize};
                }
                else
                {
                    break;
                }
                lastChuckSize = currentChuckSize;
                sizeRange.Add(chuck);
            }
        }
        public string SongUrl
        {
            get
            {
                if (string.IsNullOrEmpty(songUrl))
                {
                    getSongFileInfo();
                }
                return songUrl;
            }
        }
        public string SongType
        {
            get
            {
                if (songType.Equals(string.Empty))
                {
                    getSongFileInfo();
                }
                return songType;
            }
        }
        public Lrcs GetLrc
        {
            get
            {
                if (lrc == null)
                {
                    object? content = Api.Lyric.Lrc(Id, ResEntry.ncm).Property("lrc");
                    if (content != null)
                    {
                        lrcString = ((JProperty)content).Value["lyric"].ToString();
                        lrc = new(lrcString);
                    }
                    else
                    {
                        lrcString = "[99:99.000] 暂无歌词";
                        lrc = new(lrcString);
                    }
                }
                return lrc;
            }
        }
        public string GetLrcString
        {
            get => lrcString;
        }
        public string[] Artists
        { 
            get {
                return artists; 
            } 
        }
        public string AlbumName
        { get { return albumName; } }
        public string AlbumId
        { get { return albumId; } }
        public TimeSpan DuartionTime
        {
            get => duartionTime;
        }
        public string DuartionTimeString
        {
            get => duartionTimeString;
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
            int minMs = int.Parse(timeString.Split(":")[0]) * 60 * 1000;
            int secMs = int.Parse(timeString.Split(":")[1].Split(".")[0]) * 1000;
            int ms = int.Parse(timeString.Split(":")[1].Split(".")[1]);
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