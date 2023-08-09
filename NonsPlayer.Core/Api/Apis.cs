using Newtonsoft.Json.Linq;
using NonsPlayer.Core.Enums;
using RestSharp;

namespace NonsPlayer.Core.Api;

public static class Apis
{
    public static class Playlist
    {
        public static async Task<JObject> Detail(long? id, Nons nons)
        {
            var _URL = "https://music.163.com/api/v6/playlist/detail";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"id", id.ToString()},
                {"n", "100000"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Personalized(Nons nons, int limit = 30)
        {
            var _URL = "https://music.163.com/api/personalized/playlist";
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
            var dataString = "[";
            for (var index = 0; index <= ids.Length - 1; index++)
                if (index != ids.Length - 1)
                    dataString += "{id:" + ids[index] + "},";
                else
                    dataString += "{id:" + ids[index] + "}]";

            var _URL = "https://music.163.com/api/v3/song/detail";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"c", dataString}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Url(long id, MusicQualityLevel level, Nons nons)
        {
            var _URL = "https://interface.music.163.com/api/song/enhance/player/url/v1";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"ids", $"[{id}]"},
                {"level", MusicQualityLevelConvert.ToQualityString(level)},
                {"encodeType", "flac"}
            };
            if (level is MusicQualityLevel.Sky)
            {
                pairs.Add("immerseType", "c51");
            }

            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Like(string id, bool like, Nons nons)
        {
            var _URL = "https://music.163.com/api/radio/like";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"alg", "itembased"},
                {"trackId", id},
                {"like", like},
                {"time", "3"}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> LikeList(string uid, Nons nons)
        {
            var _URL = "https://music.163.com/api/song/like/get";
            var pairs = new Dictionary<string, object>
            {
                {"uid", uid}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Lyric
    {
        public static async Task<JObject> GetLyric(string id, Nons nons)
        {
            var _URL = "https://music.163.com/api/song/lyric?_nmclfl=1";
            var resBody = $"id={id}&tv=-1&lv=-1&rv=-1&kv=-1";
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
            var _URL = "https://music.163.com/api/login";
            var pswMD5 = nons?.Md5(password);
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
            var _URL = "https://music.163.com/api/login/cellphone";
            var pswMD5 = nons.Md5(password);
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
            var _URL = "https://music.163.com/api/login/cellphone";
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
                var _URL = "https://music.163.com/api/login/qrcode/unikey?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    {"timestamp", timestamp}
                };
                return await nons.Request(_URL, pairs);
            }

            public static async Task<JObject> Creat(string key, string timestamp, Nons nons)
            {
                var _URL = "https://music.163.com/login?type=1";
                IDictionary<string, object> pairs = new Dictionary<string, object>
                {
                    {"timestamp", timestamp},
                    {"codekey", key}
                };
                return await nons.Request(_URL);
            }

            public static async Task<IRestResponse> Check(string key, Nons nons)
            {
                var _URL = $"https://music.163.com/api/login/qrcode/client/login?type=1&key={key}";
                return await nons.RequestRestResponse(_URL);
            }
        }
    }

    public static class User
    {
        public static async Task<JObject> Detail(string id, Nons nons)
        {
            var _URL = $"https://music.163.com/api/v1/user/detail/{id}";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> Account(Nons nons)
        {
            var _URL = "https://music.163.com/api/nuser/account/get";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> DailyTask(string type, Nons nons)
        {
            var _URL = "https://music.163.com/api/point/dailyTask";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"type", type}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Likelist(string id, Nons nons)
        {
            var _URL = "https://music.163.com/api/song/like/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"uid", id}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Playlist(string uid, Nons nons, int limit = 30, int offset = 0,
            bool includeVideo = false)
        {
            var _URL = "https://music.163.com/api/user/playlist";
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
            var _URL = "https://music.163.com/weapi/v1/discovery/recommend/resource";
            return await nons.Request(_URL);
        }

        public static async Task<JObject> Musics(Nons nons)
        {
            var _URL = "https://music.163.com/api/v3/discovery/recommend/songs";
            return await nons.Request(_URL);
        }
    }

    public static class Search
    {
        /// <param name="type">1: 单曲, 10: 专辑, 100: 歌手, 1000: 歌单, 1002: 用户, 1004: MV, 1006: 歌词, 1009: 电台, 1014: 视频</param>
        public static async Task<JObject> Default(string keyword, int limit, int type, Nons nons)
        {
            var _URL = "https://music.163.com/api/search/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"s", keyword},
                {"type", type},
                {"limit", limit},
                {"offset", 0}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> Suggest(string keyword, string type, Nons nons)
        {
            var _URL = $"https://music.163.com/api/search/suggest/{type}";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"s", keyword}
            };
            return await nons.Request(_URL, pairs);
        }

        public static async Task<JObject> MultiMatch(string keyword, Nons nons)
        {
            var _URL = "https://music.163.com/api/search/suggest/multimatch";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"s", keyword}
            };
            return await nons.Request(_URL, pairs);
        }
    }

    public static class Artist
    {
        public static async Task<JObject> Detail(long id, Nons nons)
        {
            var _URL = "https://music.163.com/api/artist/head/info/get";
            IDictionary<string, object> pairs = new Dictionary<string, object>
            {
                {"id", id}
            };
            return await nons.Request(_URL, pairs);
        }
    }
}