using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using RestSharp;

namespace NonsPlayer.Core.Utils;

public static class HttpUtils
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

    public static async Task<string> HttpGetAsync(string url, string contentType = "application/json", Dictionary<string, string> headers = null)
    {
        using (HttpClient client = new HttpClient())
        {
            if (contentType != null)
                client.DefaultRequestHeaders.Add("ContentType", contentType);
            if (headers != null)
            {
                foreach (var header in headers)
                    client.DefaultRequestHeaders.Add(header.Key, header.Value);
            }
            HttpResponseMessage response = await client.GetAsync(url);
            return await response.Content.ReadAsStringAsync();
        }
    }
}