using NcmApi.Utils;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;

namespace NcmApi;

public class Ncm
{
    private HttpClient _httpClient = new HttpClient();
    private string _token = string.Empty;

    public bool isLoggedin
    {
        get
        {
            if (!_token.Equals(string.Empty))
            {
                return true;
            }
            else
            {
                return false;
            }
        }
    }

    public string Token
    {
        get
        {
            return _token;
        }
    }

    public Ncm()
    {
    }

    #region 登录

    public void Login(string token)
    {
        _token = token;
    }

    public void Login(string email, string password)
    {
        JObject result = Api.Login.Email(email, password, this).Result;
        if ((int)result["code"] == 502)
        {
            throw new LoginFailed("账号或密码错误");
        }
        else
        {
            _token = result["token"].ToString();
        }
    }

    public void Login(int phone, string password)
    {
        JObject result = Api.Login.PhonePsw(phone, password, this).Result;
        if ((int)result["code"] == 502)
        {
            throw new LoginFailed("账号或密码错误");
        }
        else
        {
            _token = result["token"].ToString();
        }
    }

    public void Login(int phone, int captcha)
    {
        JObject result = Api.Login.PhoneVer(phone, captcha, this).Result;
        if ((int)result["code"] == 502)
        {
            throw new LoginFailed("账号或密码错误");
        }
        else
        {
            _token = result["token"].ToString();
        }
    }

    #endregion 登录

    public async Task<JObject> Request(HttpMethod method, string url, HttpContent? postData = null)
    {
        HttpRequestMessage msg = new(method, url);
        List<string> cookie = new List<string>();
        if (method == HttpMethod.Post)
        {
            cookie.Add("os=pc");
            cookie.Add("appver=2.9.7");
        }
        if (!_token.Equals(string.Empty))
        {
            cookie.Add($"MUSIC_U={_token}");
        }
        if (postData != null)
        {
            msg.Content = postData;
            msg.Content.Headers.ContentType = new MediaTypeHeaderValue("application/x-www-form-urlencoded");
        }
        if (url.Contains("music.163.com"))
        {
            msg.Headers.Referrer = new Uri("https://music.163.com");
        }
        string cookies = string.Empty;
        foreach (string item in cookie)
        {
            cookies += item + ";";
        }
        msg.Headers.Add("Cookie", cookies);
        _httpClient.DefaultRequestHeaders.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        var task = _httpClient.SendAsync(msg);
        JObject result = JObject.Parse(task.Result.Content.ReadAsStringAsync().Result);
        return result;
    }

    public string MD5(string content)
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

    public HttpContent DefaultContent(HttpContent content)
    {
        HttpContent result = content;
        result.Headers.Add("Cookie", "os=pc;appver=2.9.7");
        return result;
    }
}

public static class Api
{
    public static class Playlist
    {
        public static async Task<JObject> Detail(long id, Ncm ncm)
        {
            string _URL = $"https://music.163.com/api/v6/playlist/detail";
            HttpContent data = new StringContent($"id={id}&n=100000");
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static async Task<JObject> Personalized(Ncm ncm, int limit = 30)
        {
            string _URL = $"https://music.163.com/api/personalized/playlist";
            HttpContent data = new StringContent($"limit={limit}&total=true&n=1000");
            return await ncm.Request(HttpMethod.Post, _URL, data);
        }
    }

    public static class Music
    {
        public static async Task<JObject> Detail(long[] ids, Ncm ncm)
        {
            string dataString = "[";
            for (int index = 0; index <= ids.Length - 1; index++)
            {
                if (index != ids.Length - 1)
                {
                    dataString += "{id:" + ids[index].ToString() + "},";
                }
                else
                {
                    dataString += "{id:" + ids[index].ToString() + "}]";
                }
            }
            string _URL = $"https://music.163.com/api/v3/song/detail";
            HttpContent data = new StringContent($"c={dataString}");
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static async Task<JObject> Url(long[] ids, Ncm ncm)
        {
            string _URL = "http://music.163.com/api/song/enhance/player/url";
            string idsBody = "ids=[";
            for (int index = 0; index <= ids.Length - 1; index++)
            {
                if (index != ids.Length - 1)
                {
                    idsBody += ids[index].ToString() + ", ";
                }
                else
                {
                    idsBody += ids[index].ToString() + "]";
                }
            }
            string resBody = $"{idsBody}&br=999000";
            HttpContent data = new StringContent(resBody);
            string ntesNuid;
            if (!ncm.isLoggedin)
            {
                ntesNuid = new Random().RandomBytes(16).ToHexStringLower();
                data.Headers.Add("Cookie", $"_ntes_nuid={ntesNuid}");
            }
            return await ncm.Request(HttpMethod.Post, _URL, data);
        }

        public static async Task<JObject> Like(string id, bool like, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/radio/like";
            HttpContent data = new StringContent($"alg=itembased&trackId={id}&like={like}&time=3");
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }
    }

    public static class Lyric
    {
        public static async Task<JObject> Lrc(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/lyric?_nmclfl=1";
            string resBody = $"id={id}&tv=-1&lv=-1&rv=-1&kv=-1";
            HttpContent data = new StringContent(resBody);
            return await ncm.Request(HttpMethod.Post, _URL, data);
        }
    }

    public static class Login
    {
        public static async Task<JObject> Email(string email, string password, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/login";
            string pswMD5 = ncm.MD5(password);
            string resBody = $"username={email}&password={pswMD5}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static async Task<JObject> PhonePsw(int phone, string password, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string pswMD5 = ncm.MD5(password);
            string resBody = $"phone={phone}&countryCode={countryCode}&captcha={pswMD5}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static async Task<JObject> PhoneVer(int phone, int captcha, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string resBody = $"phone={phone}&countryCode={countryCode}&captcha={captcha}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return await ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }
    }

    public static class User
    {
        public static async Task<JObject> Detail(string id, Ncm ncm)
        {
            string _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return await ncm.Request(HttpMethod.Post, _URL);
        }

        public static async Task<JObject> Account(Ncm ncm)
        {
            string _URL = "https://music.163.com/api/nuser/account/get";
            return await ncm.Request(HttpMethod.Post, _URL);
        }

        public static async Task<JObject> DailyTask(string type, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/point/dailyTask";
            HttpContent data = new StringContent($"type={type}");
            return await ncm.Request(HttpMethod.Post, _URL, data);
        }

        public static async Task<JObject> Likelist(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/like/get";
            HttpContent data = new StringContent($"uid={id}");
            return await ncm.Request(HttpMethod.Post, _URL, data);
        }
    }

    public static class Recommend
    {
        public static async Task<JObject> Resource(Ncm ncm)
        {
            string _URL = "https://music.163.com/weapi/v1/discovery/recommend/resource";
            return await ncm.Request(HttpMethod.Post, _URL);
        }

        public static async Task<JObject> Musics(Ncm ncm)
        {
            string _URL = "https://music.163.com/api/v3/discovery/recommend/songs";
            return await ncm.Request(HttpMethod.Post, _URL);
        }
    }
}