using NcmApi;
using NcmPlayer.Resources;
using Newtonsoft.Json.Linq;

namespace NcmPlayer.Framework.Model
{
    public class PlayList
    {
        /// <summary>
        /// 歌单名称
        /// </summary>
        public string Name
        {
            get; set;
        }

        /// <summary>
        /// 歌单Id
        /// </summary>
        public long Id
        {
            get; set;
        }

        /// <summary>
        /// 歌单封面Url
        /// </summary>
        public string PicUrl
        {
            get; set;
        }

        /// <summary>
        /// 歌单描述
        /// </summary>
        public string Description
        {
            get; set;
        }

        /// <summary>
        /// 歌单标签
        /// </summary>
        public string[] Tags;

        /// <summary>
        /// 歌单创建者
        /// </summary>
        public string Creator
        {
            get; set;
        }

        /// <summary>
        /// 歌单创建时间
        /// </summary>
        public DateTime CreateTime;

        /// <summary>
        /// 歌单歌曲全部Id
        /// </summary>
        public long[] MusicTrackIds;

        /// <summary>
        /// 歌单歌曲数量
        /// </summary>
        public int MusicsCount => MusicTrackIds.Length;

        /// <summary>
        /// 通过Id打开歌单
        /// </summary>

        public async Task LoadAsync(long in_id)
        {
            Id = in_id;
            JObject playlistDetail = (JObject)(await Api.Playlist.Detail(Id, ResEntry.ncm))["playlist"];
            Name = playlistDetail["name"].ToString();
            Description = playlistDetail["description"].ToString();

            JArray jsonTags = (JArray)playlistDetail["tags"];
            Tags = new string[jsonTags.Count];
            for (int index = 0; index < Tags.Length; index++)
            {
                Tags[index] = jsonTags[index].ToString();
            }

            PicUrl = playlistDetail["coverImgUrl"].ToString();
            JArray jsonMusics = (JArray)playlistDetail["trackIds"];
            MusicTrackIds = new long[jsonMusics.Count];
            for (int index = 0; index < MusicTrackIds.Length; index++)
            {
                MusicTrackIds[index] = (int)jsonMusics[index]["id"];
            }
            string timestampTemp = playlistDetail["createTime"].ToString();
            CreateTime = Tool.TimestampToDateTime(timestampTemp.Remove(timestampTemp.Length - 3));
            Creator = playlistDetail["creator"]["nickname"].ToString();
        }

        public async Task<Music[]> InitArtWorkList(int start = 0, int end = 0)
        {
            JArray musicDetail;
            if (end != 0)
            {
                musicDetail = (JArray)(await Api.Music.Detail(MusicTrackIds[start..end], ResEntry.ncm))["musics"];
            }
            else
            {
                if (MusicTrackIds.Length >= 500)
                {
                    var temp = await Api.Music.Detail(MusicTrackIds[0..500], ResEntry.ncm);
                    musicDetail = (JArray)temp["songs"];
                }
                else
                {
                    var temp = await Api.Music.Detail(MusicTrackIds, ResEntry.ncm);
                    musicDetail = (JArray)temp["songs"];
                }
            }

            Music[] musics = new Music[musicDetail.Count];
            for (int index = 0; index < musics.Length; index++)
            {
                musics[index] = new Music((JObject)musicDetail[index]);
            }
            return musics;
        }

        public async Task<Stream> GetPic(int x = 0, int y = 0)
        {
            string IMGSIZE = $"?param=300y300";
            if (x == 0)
            {
                return await HttpRequest.StreamHttpGet(PicUrl + IMGSIZE);
            }
            else
            {
                return await HttpRequest.StreamHttpGet(PicUrl + $"?param={x}y{y}");
            }
        }
    }
}