using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using RestSharp;
using NonsPlayer.Core.Account;

namespace NonsPlayer.Core.Api;

public class Nons
{
    public static Nons Instance
    {
        get;
    } = new();

    
    private RestClient _client = new("https://music.163.com");
    
    public async Task<IRestResponse> RequestRestResponse(string url, IDictionary<string, object>? parameters = null)
    {
        var request = new RestRequest(url, Method.POST);
        if (parameters != null)
        {
            foreach (var keyValuePair in parameters)
            {
                request.AddParameter(keyValuePair.Key, keyValuePair.Value);
            }
        }

        request.AddCookie("os", "pc");
        request.AddCookie("appver", "2.9.7");
        request.AddHeader("ContentType", "application/x-www-form-urlencoded");
        request.AddHeader("UserAgent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        request.AddHeader("Referrer", "https://music.163.com");

        if (!Account.Account.Instance.Token.Equals(string.Empty))
        {
            request.AddCookie("MUSIC_U", Account.Account.Instance.Token);
        }

        return await _client.ExecuteAsync(request);
    }

    public async Task<JObject> Request(string url, IDictionary<string, object>? parameters = null)
    {
        var respResult = (await RequestRestResponse(url, parameters)).Content;
        var result = JObject.Parse(respResult);
        return result;
    }

    public async Task<JObject[]> RequestMultiple(IEnumerable<Task<JObject>> tasks)
    {
        return await Task.WhenAll(tasks);
    }

    public string Md5(string content)
    {
        MD5 md5 = new MD5CryptoServiceProvider();
        byte[] fromData = Encoding.UTF8.GetBytes(content);
        byte[] targetData = md5.ComputeHash(fromData);
        string byte2String = string.Empty;

        for (int i = 0; i < targetData.Length; i++)
        {
            byte2String = byte2String + targetData[i].ToString("x2");
        }

        return byte2String;
    }
}