using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Enums;
using NonsPlayer.Core.Nons;
using RestSharp;

namespace NonsPlayer.Core.Api;

public static class Apis
{
    public static class Playlist
    {
        public static async Task<JObject> Detail(long? id, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/v6/playlist/detail";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "id", id.ToString() },
                { "n", "100000" }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Subscribe(long id, bool isSubscribe, NonsCore NonsCore)
        {
            var _URL = $"https://music.163.com/api/playlist/{(isSubscribe ? "subscribe" : "unsubscribe")}";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "id", id }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Music
    {
        public static async Task<JObject> Detail(long[] ids, NonsCore NonsCore)
        {
            var dataString = "[";
            for (var index = 0; index <= ids.Length - 1; index++)
                if (index != ids.Length - 1)
                    dataString += "{id:" + ids[index] + "},";
                else
                    dataString += "{id:" + ids[index] + "}]";

            var _URL = "https://music.163.com/api/v3/song/detail";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "c", dataString }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Url(long id, MusicQualityLevel level, NonsCore NonsCore)
        {
            var _URL = "https://interface.music.163.com/api/song/enhance/player/url/v1";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "ids", $"[{id}]" },
                { "level", MusicQualityLevelConvert.ToQualityString(level) },
                { "encodeType", "flac" }
            };
            if (level is MusicQualityLevel.Sky) pairs.Add("immerseType", "c51");

            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Like(string id, bool like, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/radio/like";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "alg", "itembased" },
                { "trackId", id },
                { "like", like },
                { "time", "3" }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> LikeList(string uid, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/song/like/get";
            var pairs = new Dictionary<string, object>
            {
                { "uid", uid }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Lyric
    {
        public static async Task<JObject> GetLyric(string id, NonsCore NonsCore)
        {
            var _URL = "https://interface3.music.163.com/api/song/lyric/v1";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "id", id },
                { "lv", 0 },
                { "kv", 0 },
                { "tv", 0 },
                { "yv", 0 },
                { "rv", 0 },
                { "ytv", 0 },
                { "yrv", 0 }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Login
    {
        public static async Task<JObject> Email(string email, string password, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/login";
            var pswMD5 = NonsCore?.Md5(password);
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "username", email },
                { "password", pswMD5 },
                { "rememberLogin", "true" }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> PhonePsw(int phone, string password, NonsCore NonsCore, int countryCode = 86)
        {
            var _URL = "https://music.163.com/api/login/cellphone";
            var pswMD5 = NonsCore.Md5(password);
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "phone", phone.ToString() },
                { "countryCode", countryCode.ToString() },
                { "captcha", pswMD5 },
                { "rememberLogin", "true" }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> PhoneVer(int phone, int captcha, NonsCore NonsCore, int countryCode = 86)
        {
            var _URL = "https://music.163.com/api/login/cellphone";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "phone", phone.ToString() },
                { "countryCode", countryCode.ToString() },
                { "rememberLogin", "true" }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static class QRCode
        {
            public static async Task<JObject> Key(string timestamp, NonsCore NonsCore)
            {
                var _URL = "https://music.163.com/api/login/qrcode/unikey?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    { "timestamp", timestamp }
                };
                return await NonsCore.Request(_URL, pairs);
            }

            public static async Task<JObject> Creat(string key, string timestamp, NonsCore NonsCore)
            {
                var _URL = "https://music.163.com/login?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    { "timestamp", timestamp },
                    { "codekey", key }
                };
                return await NonsCore.Request(_URL);
            }

            public static async Task<IRestResponse> Check(string key, NonsCore NonsCore)
            {
                var _URL = $"https://music.163.com/api/login/qrcode/client/login?type=1&key={key}";
                return await NonsCore.RequestRestResponse(_URL);
            }
        }
    }

    public static class User
    {
        public static async Task<JObject> Detail(string id, NonsCore NonsCore)
        {
            var _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return await NonsCore.Request(_URL);
        }

        public static async Task<JObject> Account(NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/nuser/account/get";
            return await NonsCore.Request(_URL);
        }

        public static async Task<JObject> DailyTask(string type, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/point/dailyTask";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "type", type }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Likelist(string id, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/song/like/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "uid", id }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Playlist(string uid, NonsCore NonsCore, int limit = 30, int offset = 0,
            bool includeVideo = false)
        {
            var _URL = "https://music.163.com/api/user/playlist";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "uid", uid },
                { "limit", limit },
                { "offset", offset },
                { "includeVideo", includeVideo }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Recommend
    {
        public static async Task<JObject> Resource(NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/weapi/v1/discovery/recommend/resource";
            return await NonsCore.Request(_URL);
        }

        public static async Task<JObject> Musics(NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/v3/discovery/recommend/songs";
            return await NonsCore.Request(_URL);
        }

        public static async Task<JObject> Playlist(NonsCore NonsCore, int limit = 30)
        {
            var _URL = "https://music.163.com/api/personalized/playlist";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "limit", limit.ToString() },
                { "total", "true" },
                { "n", "1000" }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Search
    {
        /// <param name="type">1: 单曲, 10: 专辑, 100: 歌手, 1000: 歌单, 1002: 用户, 1004: MV, 1006: 歌词, 1009: 电台, 1014: 视频</param>
        public static async Task<JObject> Default(string keyword, int limit, int type, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/search/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "s", keyword },
                { "type", type },
                { "limit", limit },
                { "offset", 0 }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> Suggest(string keyword, string type, NonsCore NonsCore)
        {
            var _URL = $"https://music.163.com/api/search/suggest/{type}";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "s", keyword }
            };
            return await NonsCore.Request(_URL, pairs);
        }

        public static async Task<JObject> MultiMatch(string keyword, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/search/suggest/multimatch";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "s", keyword }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }

    public static class Artist
    {
        public static async Task<JObject> Detail(long id, NonsCore NonsCore)
        {
            var _URL = "https://music.163.com/api/artist/head/info/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                { "id", id }
            };
            return await NonsCore.Request(_URL, pairs);
        }
    }
}