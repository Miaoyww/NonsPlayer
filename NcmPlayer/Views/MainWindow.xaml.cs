using Microsoft.Win32;
using NcmPlayer.Resources;
using System;
using System.Timers;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using Wpf.Ui.Appearance;
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

        public MainWindow()
        {
            InitializeComponent();
            acc = this;
            PublicMethod.Init(this);

            btn_albumPic.DataContext = Res.res;
            tblock_artists.DataContext = Res.res;
            tblock_title.DataContext = Res.res;
            slider_volume.DataContext = Res.res;
            label_currentTime.DataContext = Res.res;
            label_wholeTime.DataContext = Res.res;
            slider_postion.DataContext = Res.res;

            timer.Interval = 100;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();
            isUser = true;
        }

        private void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                // ui更新
                if (Res.res.IsPlaying)
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
            PublicMethod.ChangePage(PublicMethod.PageExplore);
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            Res.wholePlaylist.Last();
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            MusicPlayer.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            Res.wholePlaylist.Next();
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
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
            PublicMethod.ChangePage(PublicMethod.PageSettings);
        }

        private void slider_volume_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                Regediter.Regedit("CurrentVolume", ((int)slider_volume.Value).ToString());
                Res.res.CVolume = (int)slider_volume.Value;
            }
        }

        private void btn_volume_Click(object sender, RoutedEventArgs e)
        {
            if (Res.res.CVolume != 0)
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
            Res.res.ScreenSize = new double[] { ScreenFrame.RenderSize.Width, ScreenFrame.RenderSize.Height };
            Res.res.PageSize = new double[] { PageFrame.RenderSize.Width, PageFrame.RenderSize.Height };
        }

        private void slider_postion_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUserPostion)
            {
                MusicPlayer.Postion((int)slider_postion.Value);
            }
        }

        private void slider_postion_PreviewMouseDown(object sender, System.Windows.Input.MouseEventArgs e)
        {
            isUserPostion = true;
        }

        private void slider_postion_PreviewMouseUp(object sender, System.Windows.Input.MouseButtonEventArgs e)
        {
            isUserPostion = false;
        }

        private void btn_showPlaylist_Click(object sender, RoutedEventArgs e)
        {
            PublicMethod.PlaylistBarControl();
        }
    }
}