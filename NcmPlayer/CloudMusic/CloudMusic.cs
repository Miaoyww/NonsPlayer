using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.IO;
using System.Net;
using System.Threading;

namespace NcmPlayer.CloudMusic
{
    public static class HttpRequest
    {
        public static Stream StreamHttpGet(string url)
        {
            WebRequest wrGETURL;
            wrGETURL = WebRequest.Create(url);

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
    }

    public static class Apis
    {
        public static string playListDetail(string id)
        {
            return AppConfig.ApiUrl + "/playlist/detail?id=" + id;
        }

        public static string songUrl(string id)
        {
            return AppConfig.ApiUrl + "/song/url?id=" + id;
        }

        public static string songDetail(string id)
        {
            return AppConfig.ApiUrl + "/song/detail?ids=" + id;
        }
    }

    public class CloudMusic
    {
        private string id = string.Empty;
        private string name = string.Empty;
        private Stream? cover;
        private string coverUrl;

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
                if (cover == null)
                {
                    cover = HttpRequest.StreamHttpGet(coverUrl);
                }
                if (!cover.CanRead)
                {
                    cover = HttpRequest.StreamHttpGet(coverUrl);
                }
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
        private DateTime createTime;
        private string creator = String.Empty;
        private Song[] songs;
        private bool[] threadDone;

        public PlayList(string in_id)
        {
            Id = in_id;
            JObject playlistDetail = (JObject)HttpRequest.GetJson(Apis.playListDetail(Id))["playlist"];
            Name = playlistDetail["name"].ToString();
            description = playlistDetail["description"].ToString();

            JArray jsonTags = (JArray)playlistDetail["tags"];
            tags = new string[jsonTags.Count];
            for (int index = 0; index < tags.Length; index++)
            {
                tags[index] = jsonTags[index].ToString();
            }

            CoverUrl = playlistDetail["coverImgUrl"].ToString();
            Cover = HttpRequest.StreamHttpGet(CoverUrl);

            JArray jsonSongs = (JArray)playlistDetail["trackIds"];
            songsId = new string[jsonSongs.Count];
            for (int index = 0; index < songsId.Length; index++)
            {
                songsId[index] = jsonSongs[index]["id"].ToString();
            }

            string timestampTemp = playlistDetail["createTime"].ToString();
            createTime = Tool.TimestampToDateTime(timestampTemp.Remove(timestampTemp.Length - 3));
            creator = playlistDetail["creator"]["nickname"].ToString();
        }

        public Song[] InitArtWorkList(int start, int end)
        {
            int maxWorkerThreads = 10;
            ThreadPool.SetMaxThreads(maxWorkerThreads, maxWorkerThreads);
            string[] ids = SongsId[start..end];
            songs = new Song[end - start];
            threadDone = new bool[end - start];
            for (int index = 0; index < threadDone.Length; index++)
            {
                threadDone[index] = false;
            }
            for (int index = 0; index < songs.Length; index++)
            {
                ThreadPool.QueueUserWorkItem(new WaitCallback(GetSongDetail), new object[] { ids[index], index });
            }
            while (true)
            {
                bool isExsit = true;
                foreach (bool state in threadDone)
                {
                    if (!state)
                    {
                        isExsit = false;
                    }
                }
                if (isExsit)
                {
                    return songs;
                }
            }
        }

        private void GetSongDetail(object parm)
        {
            object[] objects = (object[])parm;
            Song song = new(objects[0].ToString());
            songs[(int)objects[1]] = song;
            threadDone[(int)objects[1]] = true;
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
            get => songsId.Length;
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
        private string duartionTime = string.Empty;

        public Song(string in_id)
        {
            Id = in_id;
            JObject songDetail;
            try
            {
                songDetail = (JObject)((JArray)HttpRequest.GetJson(Apis.songDetail(Id))["songs"])[0];
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

            TimeSpan timespan = TimeSpan.FromMilliseconds(int.Parse(songDetail["dt"].ToString()));
            duartionTime = timespan.ToString(@"mm\:ss");
            albumId = songDetail["al"]["id"].ToString();
            albumName = songDetail["al"]["name"].ToString();
            CoverUrl = songDetail["al"]["picUrl"].ToString();
        }

        public string[] Artists
        { get { return artists; } }

        public string AlbumName
        { get { return albumName; } }

        public string AlbumId
        { get { return albumId; } }

        public string DuartionTime
        {
            get { return duartionTime; }
        }

        public string GetFile()
        {
            string path = AppConfig.SongsPath(Id, SongType);
            if (!File.Exists(path))
            {
                if (!Directory.Exists(AppConfig.SongsDirectory))
                {
                    Directory.CreateDirectory(AppConfig.SongsDirectory);
                }
                FileStream fs = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
                Stream songStream = HttpRequest.StreamHttpGet(SongUrl);
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

        public string SongUrl
        {
            get
            {
                if (songUrl.Equals(string.Empty))
                {
                    JObject temp = (JObject)HttpRequest.GetJson(Apis.songUrl(Id))["data"][0];
                    songUrl = temp["url"].ToString();
                    songType = temp["type"].ToString();
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
                    JObject temp = (JObject)HttpRequest.GetJson(Apis.songUrl(Id))["data"][0];
                    songUrl = temp["url"].ToString();
                    songType = temp["type"].ToString();
                }
                return songType;
            }
        }
    }
}