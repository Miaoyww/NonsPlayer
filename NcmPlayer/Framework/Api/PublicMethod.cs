using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Threading;
using System.Threading.Tasks;
using NcmPlayer.Framework.Model;
using System.Windows.Controls;
using NcmPlayer.Resources;
using NcmPlayer.Views.Pages;

namespace NcmApi
{
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

        public static JObject JObjectHttpGet(Stream stream)
        {
            StreamReader objReader = new StreamReader(stream);
            return (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());
        }

        public static JObject GetJson(string url)
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

        public static async void OpenMusicListDetail(long id)
        {
            Stopwatch stopwatch = new();
            MusicListDetail newone = new()
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
                    Stream playlistCover = playList.GetPic(100, 100).Result;
                    await newone.Dispatcher.BeginInvoke(new Action(() =>
                    {
                        newone.SetCover(playlistCover);
                    }));
                    stopwatch.Stop();
                    Debug.WriteLine($"OpenMusicListDetail 获取封面耗时{stopwatch.ElapsedMilliseconds}");
                });
                getCover.IsBackground = true;
                getCover.Start();
                stopwatch.Restart();
                Music[] musics = playList.InitArtWorkList().Result;

                await newone.Dispatcher.BeginInvoke(new Action(async () =>
                {
                    await newone.UpdateMusicsList(musics, playList);
                }));
                stopwatch.Stop();
                Debug.WriteLine($"OpenMusicListDetail 更新歌曲耗时{stopwatch.ElapsedMilliseconds}");
            });
            get.Start();
        }
    }
}