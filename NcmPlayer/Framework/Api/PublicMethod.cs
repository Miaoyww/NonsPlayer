using System.Diagnostics;
using System.Net;
using NcmPlayer.Contracts.Services;
using NcmPlayer.Framework.Model;
using NcmPlayer.ViewModels;
using NcmPlayer.Views;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

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

        public static async void OpenMusicListDetail(long id, INavigationService navigationService)
        {
            navigationService.NavigateTo(typeof(MusicListDetailViewModel).FullName!, id);
        }
    }
}