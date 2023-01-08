namespace NcmPlayer.Framework.Model
{
    public class Album
    {
        /// <summary>
        /// 专辑Id
        /// </summary>
        public int Id
        {
            get; set;
        }

        /// <summary>
        /// 专辑名称
        /// </summary>
        public string Name
        {
            get; set;
        }

        /// <summary>
        /// 专辑封面
        /// </summary>
        public string PicUrl
        {
            get; set;
        }

        /// <summary>
        /// 发行时间
        /// </summary>
        public DateTime CreateDate
        {
            get; set;
        }

        /// <summary>
        /// 描述
        /// </summary>
        public string Description
        {
            get; set;
        }

        /// <summary>
        /// 专辑歌曲
        /// </summary>
        public List<Music> Musics
        {
            get; set;
        }

        /// <summary>
        /// 专辑作者
        /// </summary>
        public List<Artist> Artists
        {
            get; set;
        }

        /// <summary>
        /// 专辑作者
        /// </summary>
        public string ArtistsName
        {
            get
            {
                if (Artists == null) return string.Empty;
                return string.Join("/", Artists.Select(x => x.Name));
            }
        }

        /// <summary>
        /// 收藏数目
        /// </summary>
        public int CollectionCount
        {
            set; get;
        }

        /// <summary>
        /// 歌曲数目
        /// </summary>
        public int TrackCount
        {
            get; set;
        }
    }
}