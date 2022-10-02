using NcmPlayer.Framework.Model;
using NcmPlayer.Framework.Player;
using NcmPlayer.Views;
using NcmPlayer.Views.Pages;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using Wpf.Ui.Appearance;
using Wpf.Ui.Common;

namespace NcmPlayer.Resources
{
    // 用于储存操作主界面的一些方法与变量
    public static class PublicMethod
    {
        public static Frame Screenframe;
        public static Frame Pageframe;
        public static Frame Playlistbarframe;

        public static Page LastPage;
        public static Home PageHome = new();
        public static Settings PageSettings;
        public static Explore PageExplore;
        public static Login PageLogin = new();
        public static Player PagePlayer = new();
        public static PlayListBar PagePlaylistBar = new();
        public static Page CurrentPage;

        // 由于大部分的ui都在MainWindow中，在此传递以便初始化
        public static void Init(MainWindow window)
        {
            ResEntry.musicPlayer.InitPlayer();
            Screenframe = window.ScreenFrame;
            Pageframe = window.PageFrame;
            Playlistbarframe = window.PlayListBar;
            ChangeTheme(ResEntry.res.CurrentTheme);
            window.PageFrame.Content = PageHome;
            window.ScreenFrame.Content = PagePlayer;
            window.PlayListBar.Content = PagePlaylistBar;
        }

        public static void ShowDialog(string content, string title)
        {
            MainWindow.acc.dialog.Visibility = Visibility.Visible;
            MainWindow.acc.dialog.Title = title;
            MainWindow.acc.dialog.Content = content;
            MainWindow.acc.dialog.ButtonLeftVisibility = Visibility.Visible;
            MainWindow.acc.dialog.ButtonRightVisibility = Visibility.Visible;
            MainWindow.acc.dialog.Show();
        }

        public static void ShowDialog(object content)
        {
            MainWindow.acc.dialog.Visibility = Visibility.Visible;
            MainWindow.acc.dialog.Title = "";
            MainWindow.acc.dialog.Content = content;
            MainWindow.acc.dialog.ButtonLeftVisibility = Visibility.Hidden;
            MainWindow.acc.dialog.ButtonRightVisibility = Visibility.Hidden;
            MainWindow.acc.dialog.Background = null;
            MainWindow.acc.dialog.Show();
        }

        public static void SnackLog(string content, string title, int TimeOut = 1000, SymbolRegular icon = SymbolRegular.Info24)
        {
            if (!MainWindow.acc.snackLog.IsShown)
            {
                MainWindow.acc.snackLog.Timeout = TimeOut;
                MainWindow.acc.snackLog.Visibility = Visibility.Visible;
                MainWindow.acc.snackLog.Title = title;
                MainWindow.acc.snackLog.Message = content;
                MainWindow.acc.snackLog.Icon = icon;
                MainWindow.acc.snackLog.Show();
            }
        }

        public static void HideDialog()
        {
            MainWindow.acc.dialog.Hide();
        }

        public static void ChangePage(Page page)
        {
            CurrentPage = (Page)Pageframe.Content;
            if (page is Home)
            {
                if (CurrentPage.Equals(page))
                {
                    Home newone = new();
                    if (Screenframe.Visibility == Visibility.Visible)
                    {
                        ScreenControl();
                    }
                    Pageframe.Content = newone;
                    CurrentPage = newone;
                }
            }
            if (!CurrentPage.Equals(page))
            {
                if (Screenframe.Visibility == Visibility.Visible)
                {
                    ScreenControl();
                }
                Pageframe.Content = page;
                CurrentPage = page;
            }
        }

        public static void ScreenControl()
        {
            ResEntry.res.ScreenSize = new double[] { Screenframe.RenderSize.Width, Screenframe.RenderSize.Height };
            ResEntry.res.PageSize = new double[] { Pageframe.RenderSize.Width, Pageframe.RenderSize.Height };
            if (!ResEntry.res.isShowingPlayer)
            {
                if (Screenframe.Visibility == Visibility.Visible)
                {
                    Screenframe.RenderTransform = new TranslateTransform(0, 0);
                    Storyboard story = (Storyboard)MainWindow.acc.Resources["Hide"];
                    story.Completed += delegate
                    {
                        Screenframe.Visibility = Visibility.Hidden;
                        ResEntry.res.isShowingPlayer = false;
                    };
                    story.Begin(Screenframe);
                }
                else
                {
                    Screenframe.RenderTransform = new TranslateTransform(0, 0);
                    Storyboard story = (Storyboard)MainWindow.acc.Resources["Show"];
                    story.Completed += delegate
                    {
                        ResEntry.res.isShowingPlayer = false;
                    };
                    story.Begin(Screenframe);
                    Screenframe.Visibility = Visibility.Visible;
                }
            }
        }

        public static void PlaylistBarControl()
        {
            if (Playlistbarframe.Visibility == Visibility.Visible)
            {
                Playlistbarframe.RenderTransform = new TranslateTransform(0, 0);
                Storyboard story = (Storyboard)MainWindow.acc.Resources["Hide"];
                story.Completed += delegate
                {
                    Playlistbarframe.Visibility = Visibility.Hidden;
                };
                story.Begin(Playlistbarframe);
            }
            else
            {
                Playlistbarframe.RenderTransform = new TranslateTransform(0, 0);
                Storyboard story = (Storyboard)MainWindow.acc.Resources["Show"];
                story.Begin(Playlistbarframe);
                Playlistbarframe.Visibility = Visibility.Visible;
            }
        }

        public static Brush ConvertBrush(Stream stream)
        {
            if (stream != null)
            {
                BitmapImage image = new();
                image.BeginInit();
                image.StreamSource = stream;
                image.EndInit();
                return new ImageBrush(image);
            }
            else
            {
                return new ImageBrush(new BitmapImage(new Uri("pack://application:,,,/Assets/BackGround.png")));
            }
        }

        public static void ChangeTheme(ThemeType theme)
        {
            Theme.Apply(theme);
            ResEntry.res.CurrentTheme = theme;
            BrushConverter converter = new();
            if (theme == ThemeType.Dark)
            {
                ResEntry.res.UnfollowColor = (Brush)converter.ConvertFromString("#FFFFFF");
            }
            else if (theme == ThemeType.Light)
            {
                ResEntry.res.UnfollowColor = (Brush)converter.ConvertFromString("#000000");
            }
        }
    }
}