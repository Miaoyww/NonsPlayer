using NcmPlayer.Resources;
using NcmPlayer.Views;
using System;
using System.Timers;
using System.Windows.Controls;

namespace NcmPlayer
{
    public static class MusicPlayer
    {
        private static MediaElement player;
        public static Timer updateInfo = new();
        private static bool isInited = false;

        public static void InitPlayer()
        {
            player = new();
            player.LoadedBehavior = MediaState.Manual;
            player.UnloadedBehavior = MediaState.Manual;
            player.MediaOpened += Player_MediaOpened;
            player.MediaEnded += Player_MediaEnded;
            updateInfo.Elapsed += Timer_Elapsed;
            updateInfo.Interval = 100;
            updateInfo.Start();
        }

        public static void Reload()
        {
            try
            {
                player.Source = new Uri(ResEntry.songInfo.FilePath);
                player.Volume = ResEntry.songInfo.Volume;
                player.Position = ResEntry.songInfo.Postion;
            }
            catch
            {
            }
        }

        private static void Player_MediaEnded(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.songInfo.IsPlaying = false;
            ResEntry.wholePlaylist.Next();
        }

        private static void Player_MediaOpened(object sender, System.Windows.RoutedEventArgs e)
        {
            ResEntry.songInfo.IsPlaying = true;
        }

        // 信息更新
        public static void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
            {
                ResEntry.songInfo.Postion = player.Position;
            }));
        }

        public static void RePlay(string path, string name, string artists)
        {
            ResEntry.songInfo.Name = name;
            ResEntry.songInfo.Artists = artists;
            player.Source = new Uri(path);
            Play(true);
        }

        public static void Play(bool re = false)
        {
            try
            {
                if (re)
                {
                    ResEntry.songInfo.IsPlaying = true;
                    player.Position = TimeSpan.Zero;
                    player.Play();
                }
                else
                {
                    if (!ResEntry.songInfo.IsPlaying)
                    {
                        player.Play();
                        ResEntry.songInfo.IsPlaying = true;
                        ResEntry.songInfo.DurationTime = player.NaturalDuration.TimeSpan;
                    }
                    else
                    {
                        player.Pause();
                        ResEntry.songInfo.IsPlaying = false;
                        ResEntry.songInfo.DurationTime = player.NaturalDuration.TimeSpan;
                    }
                }
            }
            catch (InvalidOperationException)
            {
            }
        }

        public static void Volume(double volume)
        {
            if (player != null)
            {
                player.Volume = volume;
            }
        }

        public static void Position(double position)
        {
            if (ResEntry.songInfo.IsPlaying)
            {
                player.Position = TimeSpan.FromSeconds(position);
                ResEntry.songInfo.Postion = player.Position;
            }
        }
    }
}