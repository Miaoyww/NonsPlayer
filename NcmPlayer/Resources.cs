using Microsoft.Win32;
using NcmPlayer.CloudMusic;
using NcmPlayer.Player;
using NcmPlayer.Views;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Wpf.Ui.Appearance;

namespace NcmPlayer
{
    public class Resources : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged = delegate { };

        // c current
        public string cSongPath = string.Empty; // 当前播放音乐的路径

        public bool isShowingPlayer = false;
        public string serverPort = "21111"; // 开放端口 第二位 11月11日
        public string log = string.Empty;  // 运行日志, 以供诊断错误
        public string playlistPath = string.Empty; // 预留, 储存播放列表, 何以储存有待考虑

        public static void RegEditer(string key, object value)
        {
            Registry.SetValue(AppConfig.RegPath, key, value);
        }

        #region 播放列表

        private string cPlayCount = "0";

        public string CPlayCount
        {
            get
            {
                return $"共有 {cPlayCount} 首歌曲";
            }
            set
            {
                cPlayCount = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayCount"));
            }
        }

        #endregion 播放列表

        #region 主题切换事件

        private ThemeType currentTheme;

        public ThemeType CurrentTheme
        {
            get => currentTheme;
            set
            {
                if (!global::System.Collections.Generic.EqualityComparer<global::Wpf.Ui.Appearance.ThemeType>.Default.Equals(currentTheme, value))
                {
                    currentTheme = value;
                    RegEditer("Theme", value);
                    PropertyChanged(this, new PropertyChangedEventArgs("ScreenSize"));
                }
            }
        }

        #endregion 主题切换事件

        #region Page, Player Size, PlaylistBar Settings

        private double[] playlisbarSize = new double[2];

        public double[] PlaylisbarSize
        {
            get
            {
                return playlisbarSize;
            }
            set
            {
                playlisbarSize = value;
                PropertyChanged(this, new PropertyChangedEventArgs("PlaylisbarSize"));
            }
        }

        private double[] screenSize = new double[2];

        public double[] ScreenSize
        {
            get
            {
                return screenSize;
            }
            set
            {
                screenSize = value;
                PropertyChanged(this, new PropertyChangedEventArgs("ScreenSize"));
            }
        }

        private double[] pageSize = new double[2];

        public double[] PageSize
        {
            get
            {
                return pageSize;
            }
            set
            {
                pageSize = value;
                PropertyChanged(this, new PropertyChangedEventArgs("PageSize"));
            }
        }

        #endregion Page, Player Size, PlaylistBar Settings

        #region 与当前音乐有关信息

        #region 当前是否在播放

        private bool isPlaying = false;

        public bool IsPlaying
        {
            set
            {
                isPlaying = value;
                PropertyChanged(this, new PropertyChangedEventArgs("IsPlaying"));
            }
            get
            {
                return isPlaying;
            }
        }

        #endregion 当前是否在播放

        #region 当前的音量

        private int cVolume;// 当前的音量

        public int CVolume
        {
            set
            {
                cVolume = value;
                MusicPlayer.Volume((double)value / 100);
                PropertyChanged(this, new PropertyChangedEventArgs("CVolume"));
            }
            get
            {
                return cVolume;
            }
        }

        #endregion 当前的音量

        #region 当前播放音乐的名称

        private string? cPlayName = string.Empty;

        public string CPlayName
        {
            set
            {
                cPlayName = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayName"));
            }
            get
            {
                if (cPlayName.Equals(string.Empty))
                {
                    return "当前未播放音乐";
                }
                return cPlayName;
            }
        }

        #endregion 当前播放音乐的名称

        #region 当前播放音乐的音乐家们

        private string? cPlayArtists = string.Empty; // 当前播放音乐的音乐家们, 以后可以使用JObject

        public string CPlayArtists
        {
            set
            {
                cPlayArtists = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayArtists"));
            }
            get
            {
                if (cPlayArtists.Equals(string.Empty))
                {
                    return "无";
                }
                return cPlayArtists;
            }
        }

        #endregion 当前播放音乐的音乐家们

        #region 封面

        private string cPlayAlbumPicUrl;
        private string cPlayAlbumId;
        private Stream? cPlayCoverStream;  // 当前播放音乐的封面流
        private ImageBrush? cPlayCoverBrush;  // 当前播放音乐的封面Brush

        public ImageBrush? Cover(Stream? stream = null)
        {
            if (stream != null)
            {
                BitmapImage image = new();
                image.BeginInit();
                image.StreamSource = stream;
                image.EndInit();
                ImageBrush brush = new();
                brush.ImageSource = image;
                CoverStream = stream;
                CoverBrush = brush;
                return brush;
            }
            else
            {
                return cPlayCoverBrush;
            }
        }

        public Stream? CoverStream
        {
            set
            {
                cPlayCoverStream = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CoverStream"));
            }
            get
            {
                return cPlayCoverStream;
            }
        }

        public ImageBrush CoverBrush
        {
            set
            {
                cPlayCoverBrush = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CoverBrush"));
            }
            get
            {
                if (cPlayCoverBrush != null)
                {
                    return cPlayCoverBrush;
                }
                else
                {
                    return new ImageBrush(new BitmapImage(new Uri("pack://application:,,,/Assets/BackGround.png")));
                }
            }
        }

        public string CPlayAlbumPicUrl
        {
            get
            {
                return cPlayAlbumPicUrl;
            }
            set
            {
                cPlayAlbumPicUrl = value;
            }
        }

        public string CPlayAlbumId
        {
            get
            {
                return cPlayAlbumId;
            }
            set
            {
                cPlayAlbumId = value;
            }
        }

        #endregion 封面

        #region 播放进度

        // 当前播放音乐的进度
        private string cPlayPostionString = string.Empty;

        private int cPlayPostion;

        public string CPlayPostionString
        {
            set
            {
                TimeSpan timespan = TimeSpan.FromSeconds(int.Parse(value));
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                cPlayPostionString = timespan.ToString(@"mm\:ss");
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayPostionString"));
            }
            get
            {
                if (!cPlayPostionString.Equals(string.Empty))
                {
                    return cPlayPostionString;
                }
                else
                {
                    return "00:00";
                }
            }
        }

        public int CPlayPostion
        {
            set
            {
                CPlayPostionString = value.ToString();
                cPlayPostion = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayPostion"));
            }
            get
            {
                return cPlayPostion;
            }
        }

        #endregion 播放进度

        #region 音乐总时长

        // 当前播放音乐的总时长
        private string cPlayWholeTimeString = string.Empty;

        private int cPlayWholeTime;

        public string CPlayWholeTimeString
        {
            set
            {
                TimeSpan timespan = TimeSpan.FromSeconds(int.Parse(value));
                int min = timespan.Minutes;
                int sec = timespan.Seconds;
                cPlayWholeTimeString = timespan.ToString(@"mm\:ss");
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayWholeTimeString"));
            }
            get
            {
                if (!cPlayWholeTimeString.Equals(string.Empty))
                {
                    return cPlayWholeTimeString;
                }
                else
                {
                    return "00:00";
                }
            }
        }

        public int CPlayWholeTime
        {
            set
            {
                CPlayWholeTimeString = value.ToString();
                cPlayWholeTime = value;
                PropertyChanged(this, new PropertyChangedEventArgs("CPlayWholeTime"));
            }
            get
            {
                return cPlayWholeTime;
            }
        }

        #endregion 音乐总时长

        #endregion 与当前音乐有关信息
    }

    public class SongVis
    {
        private int selfIndex;
        public int Index
        {
            set
            {
                selfIndex = value;
            }
            get
            {
                return selfIndex;
            }
        }
        public string Id { get; set; }
        public Song Song;
        public ListBoxItem View { get; set; }
    }

    public class PlayList
    {
        private int index;

        public int Index
        {
            set
            {
                index = value;
            }
            get
            {
                return index;
            }
        }

        public int Count { get => list.Count; }

        private List<SongVis> list = new List<SongVis>();

        public SongVis Song2Vis(Song song)
        {
            SongVis songvis = new();
            songvis.Song = song;
            ListBoxItem listBoxItem = new();
            Grid grid = new Grid();
            string artists = string.Empty;
            for (int i = 0; i <= song.Artists.Length - 1; i++)
            {
                if (i != song.Artists.Length - 1)
                {
                    artists += song.Artists[i] + "/";
                }
                else
                {
                    artists += song.Artists[i];
                }
            }
            TextBlock tblock_Name = new()
            {
                Text = song.Name,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14,
                Width = 130
            };
            TextBlock tblock_Artists = new()
            {
                Text = artists,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14
            };
            TextBlock tblock_Time = new()
            {
                Text = song.DuartionTime,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Center,
                FontSize = 14
            };
            grid.Children.Add(tblock_Name);
            grid.Children.Add(tblock_Artists);
            grid.Children.Add(tblock_Time);
            listBoxItem.Content = grid;
            songvis.Id = song.Id;
            songvis.View = listBoxItem;
            return songvis;
        }


        public void PostSong(SongVis song)
        {
            song.Index = list.Count - 1;
            if (IndexOf(song) == -1)
            {
                list.Add(song);
                Res.res.CPlayCount = Count.ToString();
                UpdateIndex();
            }
            
        }

        private void PlaySong(int index)
        {
            Song song = list[index].Song;
            string path = song.GetFile();
            string[] artist = song.Artists;
            string name = song.Name;
            string artists = string.Empty;
            for (int i = 0; i <= artist.Length - 1; i++)
            {
                if (i != artist.Length - 1)
                {
                    artists += artist[i] + "/";
                }
                else
                {
                    artists += artist[i];
                }
            }
            MusicPlayer.RePlay(path, name, artists, song.Cover);
            Res.res.CPlayAlbumPicUrl = song.CoverUrl;
            Res.res.CPlayAlbumId = song.AlbumId;

            Index = index;
            UpdateIndex();
        }

        public void Play(int index)
        {
            PlaySong(index);
        }

        public void Play(SongVis song)
        {
            int tempIndex = IndexOf(song);
            if (tempIndex != -1)
            {
                PlaySong(tempIndex);
            }
        }

        public void Play(string id)
        {
            int tempIndex = IndexOf(id);
            if (tempIndex != -1)
            {
                PlaySong(tempIndex);
            }
            else
            {
                PostSong(Song2Vis(new Song(id)));
                PlaySong(Count - 1);
            }
        }

        public void Last()
        {
            if (Count != 0)
            {
                int preCount = Index - 1;
                if (preCount >= 0)
                {
                    Play(preCount);
                }
                else
                {
                    Play(Count - 1);
                }
            }
        }

        public void Next()
        {
            if (Count != 0)
            {
                int preCount = Index + 1;
                if (preCount <= Count - 1)
                {
                    Play(preCount);
                }
                else
                {
                    Play(0);
                }
            }
        }

        public void Clear()
        {
            list.Clear();
        }

        public void Add(SongVis newone)
        {
            PostSong(newone);
        }

        public void Remove(SongVis removing)
        {
            list.Remove(removing);
            UpdateIndex();
        }

        public void Remove(int index)
        {
            list.RemoveAt(index);
            UpdateIndex();
        }

        public int IndexOf(SongVis song)
        {
            for (int index = 0; index < list.Count; index++)
            {
                SongVis item = list[index];
                if (item.Id == song.Id)
                {
                    return index;
                }
            }
            return -1;
        }

        public int IndexOf(string id)
        {
            for (int index = 0; index < list.Count; index++)
            {
                SongVis item = list[index];
                if (item.Id == id)
                {
                    return index;
                }
            }
            return -1;
        }

        public SongVis GetSongVis(int index)
        {
            return list[index];
        }

        private void UpdateIndex()
        {
            int lastindex = -1;
            for (int index = 0; index < list.Count; index++)
            {
                if (list[index].Index - lastindex != 1)
                {
                    list[index].Index = index;
                }
                lastindex = index;
            }
        }
    }

    // 用于访问Resources
    public static class Res
    {
        public static Resources res = new();
        public static PlayList wholePlaylist = new();
    }
}