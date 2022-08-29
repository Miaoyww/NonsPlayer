using NAudio.Wave;
using NcmPlayer.Resources;
using NcmPlayer.Views;
using System;
using System.IO;
using System.Timers;

namespace NcmPlayer
{
    public static class MusicPlayer
    {
        private static WaveOut waveOut;
        private static WaveStream mp3;
        public static Timer updateInfo = new();
        private static bool isInited = false;

        public static void InitPlayer()
        {
            waveOut = new();
            updateInfo.Elapsed += Timer_Elapsed;
            updateInfo.Interval = 100;
            updateInfo.Start();
        }

        // 信息更新
        public static void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            MainWindow.acc.Dispatcher.BeginInvoke(new Action(() =>
            {
                if (isInited && waveOut != null)
                {
                    Res.res.CPlayPostion = waveOut.GetPosition();
                }
            }));
        }

        public static void Init(Stream waveStream)
        {
            Stream ms = new MemoryStream();
                byte[] buffer = new byte[32768];
            int read;
            while ((read = waveStream.Read(buffer, 0, buffer.Length)) > 0)
            {
                ms.Write(buffer, 0, read);
            }
            mp3 = WaveFormatConversionStream.CreatePcmStream((WaveStream)ms);
            mp3.CurrentTime = TimeSpan.FromSeconds(Res.res.CPlayPostion);
            waveOut.Init(mp3);
            isInited = true;
            Res.res.IsPlaying = false;
        }

        public static void RePlay(Stream waveStream, string name, string artists)
        {
            Res.res.CPlayName = name;
            Res.res.CPlayArtists = artists;
            mp3 = WaveFormatConversionStream.CreatePcmStream((WaveStream)waveStream);
            waveOut.Init(mp3);
            isInited = true;
        }

        public static void Play(bool re = false)
        {
            if (re)
            {
                Res.res.IsPlaying = true;
                mp3.CurrentTime = TimeSpan.Zero;
                waveOut.Play();
            }
            else
            {
                if (!Res.res.IsPlaying)
                {
                    waveOut.Play();
                    Res.res.IsPlaying = true;
                }
                else
                {
                    waveOut.Pause();
                    Res.res.IsPlaying = false;
                }
            }
        }

        public static void Volume(double volume)
        {
            if (waveOut != null)
            {
                waveOut.Volume = (float)volume;
            }
        }

        public static void Postion(double postion)
        {
            if (Res.res.IsPlaying)
            {
                mp3.CurrentTime = TimeSpan.FromSeconds(postion);
            }
        }
    }
}