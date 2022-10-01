using NcmApi;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace NcmPlayer.Framework.Model
{
    public class Music
    {
        /// <summary>
        /// 歌曲名称
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// 歌曲Id
        /// </summary>
        public long Id { get; set; }

        /// <summary>
        /// 歌曲封面Url
        /// </summary>
        public string PicUrl { get; set; }

        /// <summary>
        /// 歌曲Url
        /// </summary>
        public string Url { get; set; }

        /// <summary>
        /// 下载文件的扩展名
        /// </summary>
        public string FileType { get; set; }

        /// <summary>
        /// 是否收藏歌曲
        /// </summary>
        public bool IsLiked { get; set; }

        /// <summary>
        /// 歌曲总时长
        /// </summary>
        public TimeSpan DuartionTime { get; set; }

        /// <summary>
        /// 歌曲总时长的String版 mm:ss
        /// </summary>
        public string DuartionTimeString { get; set; }

        /// <summary>
        /// 歌曲作者
        /// </summary>
        public List<Artist> Artists { get; set; }

        /// <summary>
        /// 歌曲作者名称
        /// </summary>
        public string ArtistsName => string.Join("/", Artists.Select(x => x.Name));

        /// <summary>
        /// 歌曲专辑
        /// </summary>
        public Album Album { get; set; }

        /// <summary>
        /// 歌曲专辑
        /// </summary>
        public string AlbumName => this.Album?.Name;

        public Music(JObject playlistMusicTrack, bool recommend = false)
        {
            Name = (string)playlistMusicTrack["name"];
            Id = (int)playlistMusicTrack["id"];
            PicUrl = (string)playlistMusicTrack["al"]["picUrl"];
            DuartionTime = TimeSpan.FromMilliseconds(int.Parse(playlistMusicTrack["dt"].ToString()));
            DuartionTimeString = DuartionTime.ToString(@"m\:ss");

            Album = new();
            Album.Name = (string)playlistMusicTrack["al"]["name"];
            Album.Id = (int)playlistMusicTrack["al"]["id"];
            Album.PicUrl = (string)playlistMusicTrack["al"]["picUrl"];

            Artists = new();
            JArray artists = (JArray)playlistMusicTrack["ar"];
            foreach(JObject artist in artists)
            {
                Artist one = new();
                one.Name = (string)artist["name"];
                one.Id = (int)artist["id"];
                Artists.Add(one);
            }
            Task.Run(GetFileInfo);
        }

        public Music(long in_id)
        {
            Id = in_id;
            JObject musicDetail;
            try
            {
                musicDetail = (JObject)((JArray)Api.Music.Detail(new long[] { Id }, ResEntry.ncm).Result["songs"])[0];
            }
            catch (InvalidCastException)
            {
                throw new InvalidCastException($"未能发现此音乐{in_id}");
            }
            Name = (string)musicDetail["name"];
            Id = (int)musicDetail["id"];
            PicUrl = (string)musicDetail["al"]["picUrl"];
            DuartionTime = TimeSpan.FromMilliseconds(int.Parse(musicDetail["dt"].ToString()));
            DuartionTimeString = DuartionTime.ToString(@"m\:ss");

            Album = new();
            Album.Name = (string)musicDetail["al"]["name"];
            Album.Id = (int)musicDetail["al"]["id"];
            Album.PicUrl = (string)musicDetail["al"]["picUrl"];

            Artists = new();
            JArray artists = (JArray)musicDetail["ar"];
            foreach (JObject artist in artists)
            {
                Artist one = new();
                one.Name = (string)artist["name"];
                one.Id = (int)artist["id"];
                Artists.Add(one);
            }
            Task.Run(GetFileInfo);
        }

        public async Task GetFileInfo()
        {
            JObject musicFile = (JObject)Api.Music.Url(new long[] { Id }, ResEntry.ncm).Result["data"][0];
            Url = musicFile["url"].ToString();
            FileType = musicFile["type"].ToString();
        }
        public async Task<Stream> GetPic(int x = 0, int y = 0)
        {
            string IMGSIZE = $"?param=300y300";
            if (x == 0)
            {
                return HttpRequest.StreamHttpGet(PicUrl + IMGSIZE).Result;
            }
            else
            {
                return HttpRequest.StreamHttpGet(PicUrl + $"?param={x}y{y}").Result;
            }
        }


        /*
        public string GetMp3()
        {
            string _musicUrl = MusicUrl;
            string path = AppConfig.MusicsPath(Id, MusicType);
            if (!File.Exists(path))
            {
                if (!Directory.Exists(AppConfig.MusicsDirectory))
                {
                    Directory.CreateDirectory(AppConfig.MusicsDirectory);
                }
                FileStream fs = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
                Stream musicStream = HttpRequest.StreamHttpGet(_musicUrl).Result;
                byte[] bArr = new byte[1024];
                int size = musicStream.Read(bArr, 0, bArr.Length);
                while (size > 0)
                {
                    //stream.Write(bArr, 0, size);
                    fs.Write(bArr, 0, size);
                    size = musicStream.Read(bArr, 0, bArr.Length);
                }
                fs.Close();
                musicStream.Close();
            }
            return path;
        }*/

        /*
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
        }*/
    }
}