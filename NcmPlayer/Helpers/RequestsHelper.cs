using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using NcmPlayer.Contracts.Services;
using NcmPlayer.ViewModels;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json;
using RestSharp;

namespace NcmPlayer.Helpers;
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
