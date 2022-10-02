using NcmApi;
using NcmPlayer.Framework.Model;
using NcmPlayer.Resources;
using System;
using System.Timers;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Wpf.Ui.Controls.Interfaces;
using Wpf.Ui.Mvvm.Contracts;

namespace NcmPlayer.Views
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : INavigationWindow
    {
        private bool isUser = false;
        private double lastVolume;
        private Timer timer = new Timer();
        public static string CurrentPage = string.Empty;
        public static MainWindow acc;
        private bool isUserPostion = false;
        private double positionTemp;

        public MainWindow()
        {
            InitializeComponent();
            acc = this;
            btn_like.DataContext = ResEntry.res;

            btn_albumPic.DataContext = ResEntry.musicPlayer;
            tblock_artists.DataContext = ResEntry.musicPlayer;
            tblock_title.DataContext = ResEntry.musicPlayer;
            slider_volume.DataContext = ResEntry.musicPlayer;
            label_currentTime.DataContext = ResEntry.musicPlayer;
            label_wholeTime.DataContext = ResEntry.musicPlayer;
            slider_postion.DataContext = ResEntry.musicPlayer;
            PublicMethod.Init(this);
            timer.Interval = 50;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();
            isUser = true;
        }

        private void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                // ui更新
                if (ResEntry.musicInfo.IsLiked)
                {
                    if (!btn_like.IconFilled)
                    {
                        btn_like.IconForeground = new SolidColorBrush(Color.FromArgb(200, 200, 0, 0));
                        btn_like.IconFilled = true;
                    }
                }
                else
                {
                    if (btn_like.IconFilled)
                    {
                        btn_like.IconForeground = new SolidColorBrush(Color.FromArgb(200, 10, 10, 10));
                        btn_like.IconFilled = false;
                    }
                }

                if (ResEntry.musicPlayer.IsPlaying)
                {
                    if (btn_play.Icon != Wpf.Ui.Common.SymbolRegular.Pause24)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Pause24;
                    }
                }
                else
                {
                    if (btn_play.Icon != Wpf.Ui.Common.SymbolRegular.Play24)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Play24;
                    }
                }

                if (string.IsNullOrEmpty(ResEntry.musicInfo.Name))
                {
                    if (Title != "当前未播放音乐 -NcmPlayer")
                    {
                        Title = "当前未播放音乐 -NcmPlayer";
                    }
                }
                else
                {
                    if (Title != $"{ResEntry.musicInfo.Name} - {ResEntry.musicInfo.Artists}")
                    {
                        Title = $"{ResEntry.musicInfo.Name} - {ResEntry.musicInfo.Artists}";
                    }
                }

                switch (slider_volume.Value)
                {
                    case <= 1:
                        btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.SpeakerOff24;
                        break;

                    case double n when (n >= 10 && n < 20):
                        btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker024;
                        break;

                    case double n when (n >= 20 && n < 80):
                        btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker124;
                        break;

                    case >= 80:
                        btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker224;
                        break;
                }
            }));
        }

        private void dialog_btn_click(object sender, System.Windows.RoutedEventArgs e)
        {
            dialog.Hide();
        }

        #region Windows

        protected override void OnClosed(EventArgs e)
        {
            base.OnClosed(e);
            // Make sure that closing this window will begin the process of closing the application.
            Application.Current.Shutdown();
        }

        private void mitem_openWindow_Click(object sender, RoutedEventArgs e)
        {
            this.Show();
            this.Activate();
        }

        private void mitem_close_Click(object sender, RoutedEventArgs e)
        {
            this.Close();
        }

        private void Window_Closed(object sender, EventArgs e)
        {
            OnCloseFunc.Close();
            // 以防有资源未卸载的情况
            GC.Collect();
            GC.WaitForFullGCComplete();
            Environment.Exit(0);
        }

        public Frame GetFrame()
        {
            throw new NotImplementedException();
        }

        public INavigation GetNavigation()
        {
            throw new NotImplementedException();
        }

        public bool Navigate(Type pageType)
        {
            throw new NotImplementedException();
        }

        public void SetPageService(IPageService pageService)
        {
            throw new NotImplementedException();
        }

        public void ShowWindow()
        {
            throw new NotImplementedException();
        }

        public void CloseWindow()
        {
            throw new NotImplementedException();
        }

        #endregion Windows

        private void btn_Home_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.ChangePage(PublicMethod.PageHome);
        }

        private void btn_Explore_Click(object sender, RoutedEventArgs e)
        {
            if (PublicMethod.PageExplore == null)
            {
                PublicMethod.PageExplore = new();
            }
            PublicMethod.ChangePage(PublicMethod.PageExplore);
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Last();
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            MusicPlayer.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Next();
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.wholePlaylist.Like();
        }

        private void btn_screener_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.ScreenControl();
        }

        private void mitem_login_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.ChangePage(PublicMethod.PageLogin);
        }

        private void mitem_settings_Click(object sender, RoutedEventArgs e)
        {
            if (PublicMethod.PageSettings == null)
            {
                PublicMethod.PageSettings = new();
            }
            PublicMethod.ChangePage(PublicMethod.PageSettings);
        }

        private void slider_volume_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                ResEntry.musicInfo.Volume = (int)slider_volume.Value;
            }
        }

        private void btn_volume_Click(object sender, RoutedEventArgs e)
        {
            if (ResEntry.musicInfo.Volume != 0)
            {
                lastVolume = slider_volume.Value;
                slider_volume.Value = 0;
            }
            else
            {
                slider_volume.Value = lastVolume;
            }
        }

        private void UiWindow_SizeChanged(object sender, SizeChangedEventArgs e)
        {
            ResEntry.res.ScreenSize = new double[] { ScreenFrame.RenderSize.Width, ScreenFrame.RenderSize.Height };
            ResEntry.res.PageSize = new double[] { PageFrame.RenderSize.Width, PageFrame.RenderSize.Height };
        }

        private void slider_postion_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                positionTemp = slider_postion.Value;
            }
        }

        private void slider_postion_PreviewMouseDown(object sender, System.Windows.Input.MouseEventArgs e)
        {
            // isUser = true;
            var value = (e.GetPosition(slider_postion).X / slider_postion.ActualWidth) * (slider_postion.Maximum - slider_postion.Minimum);
            ResEntry.musicPlayer.SetPosition(value);
        }

        private void slider_postion_DragStarted(object sender, System.Windows.Controls.Primitives.DragStartedEventArgs e)
        {
            slider_postion.Maximum = ResEntry.musicPlayer.DurationTimeDouble;
            slider_postion.DataContext = null;
            isUser = true;
        }

        private void slider_postion_DragCompleted(object sender, System.Windows.Controls.Primitives.DragCompletedEventArgs e)
        {
            if (!ResEntry.musicPlayer.IsPlaying)
            {
                ResEntry.musicPlayer.Play();
            }
            ResEntry.musicPlayer.SetPosition(positionTemp);
            slider_postion.DataContext = ResEntry.musicPlayer;
            isUser = false;
        }

        private void btn_showPlaylist_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.PlaylistBarControl();
        }

        private void btn_Lib_Click(object sender, RoutedEventArgs e)
        {
        }

        private void tbox_search_PreviewKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
        {
            if (e.Key == System.Windows.Input.Key.Enter)
            {
                Tool.OpenMusicListDetail(int.Parse(((TextBox)sender).Text));
            }
        }
    }
}