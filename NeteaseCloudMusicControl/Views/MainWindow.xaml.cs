using Microsoft.Win32;
using NeteaseCloudMusicControl.Views.Methods;
using System;
using System.Timers;
using System.Windows;
using System.Windows.Controls;
using Wpf.Ui.Controls.Interfaces;
using Wpf.Ui.Mvvm.Contracts;

namespace NeteaseCloudMusicControl.Views
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : INavigationWindow
    {
        private Pages.Home PageHome = new();
        private Pages.Settings PageSettings;
        private Pages.Player PagePlayer = new();
        private bool isUser = false;
        private bool isPlayed = false;
        private double lastVolume;
        private System.Timers.Timer timer = new System.Timers.Timer();
        public static string currentpage = string.Empty;
        public static Page lastPage;
        public static Frame screenframe;
        public static Frame pageframe;
        public static MainWindow mainWindow;

        public MainWindow()
        {
            InitializeComponent();
            mainWindow = this;
            screenframe = ScreenFrame;
            pageframe = PageFrame;
            timer.Interval = 100;
            timer.Elapsed += Timer_Elapsed;
            timer.Start();
        }

        private void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            Dispatcher.BeginInvoke(new Action(() =>
            {
                if (CurrentResources.isPlayed)
                {
                    if (!isPlayed)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Pause24;
                        isPlayed = true;
                    }
                }
                else
                {
                    if (isPlayed)
                    {
                        btn_play.Icon = Wpf.Ui.Common.SymbolRegular.Play24;
                        isPlayed = false;
                    }
                }

                CurrentResources.musicplayer.Dispatcher.BeginInvoke(new Action(() =>
                {
                    CurrentResources.musicplayer.Volume = slider_volume.Value / 100;
                }));
            }));
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

        private void RegEditer(string key, object value)
        {
            Registry.SetValue(AppConfig.RegPath, key, value);
        }

        public static void ScreenControl()
        {
            if (screenframe.Visibility == Visibility.Visible)
            {
                screenframe.Visibility = Visibility.Hidden;
                ChangePage(lastPage);
            }
            else
            {
                screenframe.Visibility = Visibility.Visible;
                lastPage = (Page)pageframe.Content;
                pageframe.Content = null;
                currentpage = string.Empty;
            }
        }

        private static void ChangePage(Page page)
        {
            if (!currentpage.Equals(page.Title.ToString()))
            {
                if (screenframe.Visibility == Visibility.Visible)
                {
                    screenframe.Visibility = Visibility.Hidden;
                }
                pageframe.Content = page;
                currentpage = page.Title;
            }
        }

        private void btn_Home_Click(object sender, RoutedEventArgs e)
        {
            ChangePage(PageHome);
        }

        private void btn_last_Click(object sender, System.Windows.RoutedEventArgs e)
        {
        }

        private void btn_play_Click(object sender, System.Windows.RoutedEventArgs e)
        {
            PlayerMethods.Play();
        }

        private void btn_next_Click(object sender, System.Windows.RoutedEventArgs e)
        {
        }

        private void btn_like_Click(object sender, System.Windows.RoutedEventArgs e)
        {
        }

        private void Window_Loaded(object sender, RoutedEventArgs e)
        {
            PageFrame.Content = PageHome;
            ScreenFrame.Content = PagePlayer;
            slider_volume.Value = double.Parse(CurrentResources.CurrentVolume);
            isUser = true;
        }

        private void btn_playerShow_Click(object sender, RoutedEventArgs e)
        {
            ScreenControl();
        }

        private void mitem_login_Click(object sender, RoutedEventArgs e)
        {
        }

        private void mitem_settings_Click(object sender, RoutedEventArgs e)
        {
            if (PageSettings == null)
            {
                PageSettings = new();
            }
            ChangePage(PageSettings);
        }

        private void btn_albumPic_Click(object sender, RoutedEventArgs e)
        {
            ScreenControl();
        }

        private void slider_volume_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
        {
            if (isUser)
            {
                RegEditer("CurrentVolume", slider_volume.Value);
                CurrentResources.CurrentVolume = slider_volume.Value.ToString();
            }
            switch (slider_volume.Value)
            {
                case 0:
                    btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.SpeakerOff24;
                    break;

                case double n when (n >= 10 && n < 30):
                    btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker024;
                    break;

                case double n when (n >= 30 && n < 80):
                    btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker124;
                    break;

                case >= 80:
                    btn_volume.Icon = Wpf.Ui.Common.SymbolRegular.Speaker224;
                    break;
            }
        }

        private void btn_volume_Click(object sender, RoutedEventArgs e)
        {
            if (double.Parse(CurrentResources.CurrentVolume) != 0)
            {
                lastVolume = slider_volume.Value;
                slider_volume.Value = 0;
            }
            else
            {
                slider_volume.Value = lastVolume;
            }
        }
    }
}