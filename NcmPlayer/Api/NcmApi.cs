using NcmApi.Utils;
using Newtonsoft.Json.Linq;
using System;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Security.Cryptography;
using System.Text;

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
        JObject result = Api.Login.Email(email, password, this);
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
        JObject result = Api.Login.PhonePsw(phone, password, this);
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
        JObject result = Api.Login.PhoneVer(phone, captcha, this);
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

    public JObject Request(HttpMethod method, string url, HttpContent postData = null)
    {
        HttpRequestMessage msg = new(method, url);
        if (!_token.Equals(string.Empty))
        {
            msg.Headers.Add("Cookie", $"MUSIC_U={_token}");
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
        public static JObject Detail(string id, Ncm ncm)
        {
            string _URL = $"https://music.163.com/api/v6/playlist/detail";
            HttpContent data = new StringContent($"id={id}&n=100000");
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static JObject Personalized(Ncm ncm, int limit = 30)
        {
            string _URL = $"https://music.163.com/api/personalized/playlist";
            HttpContent data = new StringContent($"limit={limit}&total=true&n=1000");
            return ncm.Request(HttpMethod.Post, _URL, data);
        }
    }

    public static class Song
    {
        public static JObject Detail(string[] ids, Ncm ncm)
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
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static JObject Url(string[] ids, Ncm ncm)
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
            return ncm.Request(HttpMethod.Post, _URL, data);
        }

        public static JObject Like(string id, bool like, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/radio/like";
            HttpContent data = new StringContent($"alg=itembased&trackId={id}&like={like}&time=3");
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }
    }

    public static class Lyric
    {
        public static JObject Lrc(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/lyric?_nmclfl=1";
            string resBody = $"id={id}&tv=-1&lv=-1&rv=-1&kv=-1";
            HttpContent data = new StringContent(resBody);
            return ncm.Request(HttpMethod.Post, _URL, data);
        }
    }

    public static class Login
    {
        public static JObject Email(string email, string password, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/login";
            string pswMD5 = ncm.MD5(password);
            string resBody = $"username={email}&password={pswMD5}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static JObject PhonePsw(int phone, string password, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string pswMD5 = ncm.MD5(password);
            string resBody = $"phone={phone}&countryCode={countryCode}&captcha={pswMD5}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }

        public static JObject PhoneVer(int phone, int captcha, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string resBody = $"phone={phone}&countryCode={countryCode}&captcha={captcha}&rememberLogin=true";
            HttpContent data = new StringContent(resBody);
            return ncm.Request(HttpMethod.Post, _URL, ncm.DefaultContent(data));
        }
    }

    public static class User
    {
        public static JObject Detail(string id, Ncm ncm)
        {
            string _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return ncm.Request(HttpMethod.Post, _URL);
        }

        public static JObject Account(Ncm ncm)
        {
            string _URL = "https://music.163.com/api/nuser/account/get";
            return ncm.Request(HttpMethod.Post, _URL);
        }

        public static JObject DailyTask(string type, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/point/dailyTask";
            HttpContent data = new StringContent($"type={type}");
            return ncm.Request(HttpMethod.Post, _URL, data);
        }
    }
}