using System.Net;
using System.Reflection.Metadata;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using RestSharp;

namespace NcmApi;

public class Ncm
{
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


    public async Task<JObject> Request(string url, IDictionary<string, string>? parameters = null)
    {
        var client = new RestClient("https://music.163.com");
        var request = new RestRequest(url, Method.POST);
        if(parameters != null){
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

        if (!_token.Equals(string.Empty))
        {
            request.AddCookie("MUSIC_U", _token);
        }
        var respResult = (await client.ExecuteAsync(request)).Content.ToString();
        JObject result = JObject.Parse(respResult);
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
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("id", id.ToString());
            pairs.Add("n", "100000");
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> Personalized(Ncm ncm, int limit = 30)
        {
            string _URL = $"https://music.163.com/api/personalized/playlist";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("limit", limit.ToString());
            pairs.Add("total", "true");
            pairs.Add("n", "1000");
            return await ncm.Request(_URL, pairs);
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
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("c", dataString);
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> Url(long[] ids, Ncm ncm)
        {
            string _URL = "http://music.163.com/api/song/enhance/player/url";
            string idsBody = "[";
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
            /*
            string ntesNuid;
            if (!ncm.isLoggedin)
            {
                ntesNuid = new Random().RandomBytes(16).ToHexStringLower();
                data.Headers.Add("Cookie", $"_ntes_nuid={ntesNuid}");
            }*/
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("ids", idsBody);
            pairs.Add("br", "999000");
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> Like(string id, bool like, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/radio/like";
            HttpContent data = new StringContent($"alg=itembased&trackId={id}&like={like}&time=3");
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("alg", "itembased");
            pairs.Add("trackId", id);
            pairs.Add("like", like.ToString());
            pairs.Add("time", "3");
            return await ncm.Request(_URL, pairs);
        }
    }

    public static class Lyric
    {
        public static async Task<JObject> Lrc(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/lyric?_nmclfl=1";
            string resBody = $"id={id}&tv=-1&lv=-1&rv=-1&kv=-1";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("id", id);
            pairs.Add("tv", "-1");
            pairs.Add("lv", "-1");
            pairs.Add("kv", "-1");
            return await ncm.Request(_URL, pairs);
        }
    }

    public static class Login
    {
        public static async Task<JObject> Email(string email, string password, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/login";
            string pswMD5 = ncm.MD5(password);
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("username", email);
            pairs.Add("password", pswMD5);
            pairs.Add("rememberLogin", "true");
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> PhonePsw(int phone, string password, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string pswMD5 = ncm.MD5(password);
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("phone", phone.ToString());
            pairs.Add("countryCode", countryCode.ToString());
            pairs.Add("captcha", pswMD5);
            pairs.Add("rememberLogin", "true");
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> PhoneVer(int phone, int captcha, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("phone", phone.ToString());
            pairs.Add("countryCode", countryCode.ToString());
            pairs.Add("rememberLogin", "true");
            return await ncm.Request(_URL, pairs);
        }
    }

    public static class User
    {
        public static async Task<JObject> Detail(string id, Ncm ncm)
        {
            string _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return await ncm.Request(_URL);
        }

        public static async Task<JObject> Account(Ncm ncm)
        {
            string _URL = "https://music.163.com/api/nuser/account/get";
            return await ncm.Request(_URL);
        }

        public static async Task<JObject> DailyTask(string type, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/point/dailyTask";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("type", type);
            return await ncm.Request(_URL, pairs);
        }

        public static async Task<JObject> Likelist(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/like/get";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("uid", id);
            return await ncm.Request(_URL, pairs);
        }
    }

    public static class Recommend
    {
        public static async Task<JObject> Resource(Ncm ncm)
        {
            string _URL = "https://music.163.com/weapi/v1/discovery/recommend/resource";
            return await ncm.Request(_URL);
        }

        public static async Task<JObject> Musics(Ncm ncm)
        {
            string _URL = "https://music.163.com/api/v3/discovery/recommend/songs";
            return await ncm.Request(_URL);
        }
    }
}