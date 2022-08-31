using Microsoft.Win32;
using NcmApi;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Wpf.Ui.Appearance;

namespace NcmPlayer.Resources
{
    public class Resources : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged = delegate { };

        // c current
        public bool isShowingPlayer = false;

        public string serverPort = "21111"; // 开放端口 第二位 11月11日
        public string log = string.Empty;  // 运行日志, 以供诊断错误
        public string playlistPath = string.Empty; // 预留, 储存播放列表, 何以储存有待考虑
        public bool allowDailySignin = false;

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
                if (!EqualityComparer<ThemeType>.Default.Equals(currentTheme, value))
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

        
    }
}