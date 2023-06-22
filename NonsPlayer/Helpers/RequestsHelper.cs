using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using RestSharp;

namespace NonsPlayer.Helpers;
public static class HttpRequest
{
    public static Stream StreamHttpGet(string url)
    {
        var client = new RestClient();
        var request = new RestRequest(url, Method.GET);
        var stream = new MemoryStream();
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
        var objReader = new StreamReader(stream);
        return (JObject)JsonConvert.DeserializeObject(objReader.ReadLine());
    }

    public static JObject? GetJson(string url)
    {
        return JObjectHttpGet(StreamHttpGet(url));
    }
}
