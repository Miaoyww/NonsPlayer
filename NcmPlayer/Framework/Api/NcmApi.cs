using System.Net;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;

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

    public async Task<JObject> Request(string url, IDictionary<string, string>? parameters = null, Encoding? charset = null)
    {
        HttpWebRequest httpReq = (HttpWebRequest)WebRequest.Create(url);
        List<string> cookie = new List<string>();
        cookie.Add("os=pc");
        cookie.Add("appver=2.9.7");
        if (!_token.Equals(string.Empty))
        {
            cookie.Add($"MUSIC_U={_token}");
        }
        if (url.Contains("music.163.com"))
        {
            httpReq.Headers.Add("Referrer", "https://music.163.com");
        }
        string cookies = string.Empty;
        foreach (string item in cookie)
        {
            cookies += item + ";";
        }
        httpReq.Headers.Add("Cookie", cookies);
        httpReq.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        httpReq.ContentType = "application/x-www-form-urlencoded";
        httpReq.Method = "POST";
        httpReq.Proxy = null;
        httpReq.KeepAlive = false;
        httpReq.AllowWriteStreamBuffering = false;
        httpReq.ServicePoint.Expect100Continue = false;
        httpReq.ServicePoint.UseNagleAlgorithm = false;

        if (!(parameters == null || parameters.Count == 0))
        {
            StringBuilder buffer = new StringBuilder();
            int i = 0;
            foreach (string key in parameters.Keys)
            {
                if (i > 0)
                {
                    buffer.AppendFormat("&{0}={1}", key, parameters[key]);
                }
                else
                {
                    buffer.AppendFormat("{0}={1}", key, parameters[key]);
                }
                i++;
            }
            byte[] data = charset.GetBytes(buffer.ToString());
            using (Stream stream = httpReq.GetRequestStream())
            {
                stream.Write(data, 0, data.Length);
            }
        }

        StreamReader sr = new StreamReader(httpReq.GetResponse().GetResponseStream()); //创建一个stream读取流
        JObject result = JObject.Parse(sr.ReadToEnd());
        httpReq.Abort();
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
        }

        public static async Task<JObject> Personalized(Ncm ncm, int limit = 30)
        {
            string _URL = $"https://music.163.com/api/personalized/playlist";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("limit", limit.ToString());
            pairs.Add("total", "true");
            pairs.Add("n", "1000");
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
        }

        public static async Task<JObject> PhoneVer(int phone, int captcha, Ncm ncm, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("phone", phone.ToString());
            pairs.Add("countryCode", countryCode.ToString());
            pairs.Add("rememberLogin", "true");
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
        }

        public static async Task<JObject> Likelist(string id, Ncm ncm)
        {
            string _URL = "https://music.163.com/api/song/like/get";
            IDictionary<string, string> pairs = new Dictionary<string, string>();
            pairs.Add("uid", id);
            return await ncm.Request(_URL, pairs, Encoding.UTF8);
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