using NcmPlayer.Contracts.Services;
using NcmPlayer.ViewModels;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using RestSharp;

namespace NcmApi;

public static class HttpRequest
{
    public static Stream StreamHttpGet(string url)
    {
        var client = new RestClient();
        var request = new RestRequest(url, Method.GET);
        MemoryStream stream = new MemoryStream();
        request.ResponseWriter = async responseStream =>
        {
            using (responseStream)
            {
                await responseStream.CopyToAsync(stream);
            }
        };
        return stream;
    }

    public static JObject? JObjectHttpGet(Stream stream)
    {
        StreamReader objReader = new StreamReader(stream);
        return (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());
    }

    public static JObject? GetJson(string url)
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

    public static void OpenMusicListDetail(long id, INavigationService navigationService)
    {
        navigationService.NavigateTo(typeof(MusicListDetailViewModel).FullName!, id);
    }
}