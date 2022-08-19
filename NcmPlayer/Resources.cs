using Microsoft.Win32;
using NcmPlayer.CloudMusic;
using NcmPlayer.Player;
using NcmPlayer.Views;
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.CompilerServices;
using System.Windows;
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
        #endregion
        #region Page与Player Size Settings

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

        #endregion
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
        #endregion
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
        #endregion
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
        #endregion
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
        #endregion
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
        #endregion
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
        #endregion
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
        #endregion
        #endregion 当前音乐有关信息
    }

    // 用于访问Resources
    public static class Res
    {
        public static Resources res = new();
    }
}