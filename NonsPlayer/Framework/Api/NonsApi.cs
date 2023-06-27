using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json.Linq;
using NonsPlayer.Framework.Api;
using RestSharp;

namespace NonsApi;

public class Nons
{
    public static Nons Instance
    {
        get;
    } = new Nons();

    private string _token = string.Empty;

    public Nons()
    {
    }

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
    public void Login(string token)
    {
        _token = token;
    }

    public void LogOut()
    {
        return;
    }

    public async Task<IRestResponse> RequestRestResponse(string url, IDictionary<string, object>? parameters = null)
    {
        var client = new RestClient("https://music.163.com");
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

        if (!_token.Equals(string.Empty))
        {
            request.AddCookie("MUSIC_U", _token);
        }

        return await client.ExecuteAsync(request);
    }

    public async Task<JObject> Request(string url, IDictionary<string, object>? parameters = null)
    {
        var respResult = (await RequestRestResponse(url, parameters)).Content;
        JObject result = JObject.Parse(respResult);
        return result;
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

public static class Api
{
    public static class Playlist
    {
        public static async Task<JObject> Detail(long id, Nons nons)
        {
            string _URL = $"https://music.163.com/api/v6/playlist/detail";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"id", id.ToString()},
                {"n", "100000"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Personalized(Nons nons, int limit = 30)
        {
            string _URL = $"https://music.163.com/api/personalized/playlist";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"limit", limit.ToString()},
                {"total", "true"},
                {"n", "1000"}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Music
    {
        public static async Task<JObject> Detail(long[] ids, Nons nons)
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
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"c", dataString}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Url(long[] ids, Nons nons)
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
            if (!nons.isLoggedin)
            {
                ntesNuid = new Random().RandomBytes(16).ToHexStringLower();
                data.Headers.Add("Cookie", $"_ntes_nuid={ntesNuid}");
            }*/
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"ids", idsBody},
                {"br", "999000"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Like(string id, bool like, Nons nons)
        {
            string _URL = "https://music.163.com/api/radio/like";
            HttpContent data = new StringContent($"alg=itembased&trackId={id}&like={like}&time=3");
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"alg", "itembased"},
                {"trackId", id},
                {"like", like.ToString()},
                {"time", "3"}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Lyric
    {
        public static async Task<JObject> GetLyric(string id, Nons nons)
        {
            string _URL = "https://music.163.com/api/song/lyric?_nmclfl=1";
            string resBody = $"id={id}&tv=-1&lv=-1&rv=-1&kv=-1";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"id", id},
                {"tv", "-1"},
                {"lv", "-1"},
                {"kv", "-1"}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Login
    {
        public static async Task<JObject> Email(string email, string password, Nons nons)
        {
            string _URL = "https://music.163.com/api/login";
            string pswMD5 = nons?.Md5(password);
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"username", email},
                {"password", pswMD5},
                {"rememberLogin", "true"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> PhonePsw(int phone, string password, Nons nons, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            string pswMD5 = nons.Md5(password);
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"phone", phone.ToString()},
                {"countryCode", countryCode.ToString()},
                {"captcha", pswMD5},
                {"rememberLogin", "true"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> PhoneVer(int phone, int captcha, Nons nons, int countryCode = 86)
        {
            string _URL = "https://music.163.com/api/login/cellphone";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"phone", phone.ToString()},
                {"countryCode", countryCode.ToString()},
                {"rememberLogin", "true"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static class QRCode
        {
            public static async Task<JObject> Key(string timestamp, Nons nons)
            {
                string _URL = "https://music.163.com/api/login/qrcode/unikey?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    {"timestamp", timestamp}
                };
                return await nons.Request(_URL, pairs);
            }

            public static async Task<JObject> Creat(string key, string timestamp, Nons nons)
            {
                string _URL = $"https://music.163.com/login?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    {"timestamp", timestamp},
                    {"codekey", key}
                };
                return await nons.Request(_URL);
            }

            public static async Task<IRestResponse> Check(string key, Nons nons)
            {
                string _URL = $"https://music.163.com/api/login/qrcode/client/login?type=1&key={key}";
                return await nons.RequestRestResponse(_URL);
            }
        }
    }

    public static class User
    {
        public static async Task<JObject> Detail(string id, Nons nons)
        {
            string _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> Account(Nons nons)
        {
            string _URL = "https://music.163.com/api/nuser/account/get";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> DailyTask(string type, Nons nons)
        {
            string _URL = "https://music.163.com/api/point/dailyTask";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"type", type}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Likelist(string id, Nons nons)
        {
            string _URL = "https://music.163.com/api/song/like/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"uid", id}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Playlist(string uid, Nons nons, int limit = 30, int offset = 0,
            bool includeVideo = false)
        {
            string _URL = "https://music.163.com/api/user/playlist";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"uid", uid},
                {"limit", limit},
                {"offset", offset},
                {"includeVideo", includeVideo}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Recommend
    {
        public static async Task<JObject> Resource(Nons nons)
        {
            string _URL = "https://music.163.com/weapi/v1/discovery/recommend/resource";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> Musics(Nons nons)
        {
            string _URL = "https://music.163.com/api/v3/discovery/recommend/songs";
            return await nons.Request(_URL);
        }
    }
}