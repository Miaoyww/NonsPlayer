using NcmPlayer.Views.Pages.Recommend;
using System.Collections.Generic;
using System.ComponentModel;
using System.Windows.Media;
using Wpf.Ui.Appearance;

namespace NcmPlayer.Resources
{
    public class Resources: INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged = delegate { };

        public bool isShowingPlayer = false;
        public bool allowDailySignin = false;
        private Brush unfollowColor = Brushes.Black;

        public Brush UnfollowColor
        {
            get => unfollowColor;
            set
            {
                unfollowColor = value;
                PropertyChanged(this, new PropertyChangedEventArgs("UnfollowColor"));
            }
        }

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
                    Regediter.Regedit("Theme", value);
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