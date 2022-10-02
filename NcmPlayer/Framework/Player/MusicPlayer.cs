using NAudio.Wave;
using NcmPlayer.Framework.Model;
using System;
using System.ComponentModel;
using System.Timers;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace NcmPlayer.Framework.Player
{
    public class MusicPlayer : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged = delegate { };

        private WaveOutEvent outputDevice;
        private MediaFoundationReader mfr;
        private Timer updateInfo = new();

        private Music musicNow;
        private ImageBrush? coverBrush;  // 当前播放音乐的封面Brush
        private int volume;// 当前的音量
        private bool isPlaying = false;
        private TimeSpan position;
        private TimeSpan durationTime;

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

        public int Volume
        {
            set
            {
                volume = value;
                PropertyChanged(this, new PropertyChangedEventArgs("Volume"));
            }
            get
            {
                return volume;
            }
        }

        public string Name
        {
            get
            {
                if (musicNow is null)
                {
                    return "当前未播放音乐";
                }
                return musicNow.Name;
            }
        }

        public string ArtistsName
        {
            get
            {
                if (musicNow is null)
                {
                    return "无";
                }
                return musicNow.ArtistsName;
            }
        }

        public ImageBrush CoverBrush
        {
            get
            {
                if (coverBrush == null)
                {
                    return new ImageBrush(new BitmapImage(new Uri("pack://application:,,,/Assets/BackGround.png")));
                }
                return coverBrush;
            }
        }

        public TimeSpan Position
        {
            set
            {
                position = value;
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(Position)));
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(PositionString)));
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(PositionDouble)));
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(Position)));
            }
            get
            {
                if (musicNow.Equals(null))
                {
                    return TimeSpan.Zero;
                }
                return position;
            }
        }

        public string PositionString
        {
            get
            {
                if (position.Equals(null))
                {
                    return "00:00";
                }
                return position.ToString(@"mm\:ss");
            }
        }

        public double PositionDouble
        {
            get
            {
                if (position.Equals(null))
                {
                    return 0.0;
                }
                return position.TotalSeconds;
            }
        }

        public string DurationTimeString
        {
            get
            {
                if (position.Equals(null))
                {
                    return "00:00";
                }
                return durationTime.ToString(@"mm\:ss"); ;
            }
        }

        public double DurationTimeDouble
        {
            get
            {
                if (durationTime.Equals(null))
                {
                    return 0.0;
                }
                return durationTime.TotalSeconds;
            }
        }

        public void InitPlayer()
        {
            outputDevice = new();
            updateInfo.Elapsed += Timer_Elapsed;
            updateInfo.Interval = 100;
            updateInfo.Start();
        }

        public void Reload()
        {
        }

        public void Timer_Elapsed(object? sender, ElapsedEventArgs e)
        {
            if (outputDevice != null)
            {
                if (outputDevice.PlaybackState == PlaybackState.Playing)
                {
                    TimeSpan convered = TimeSpan.FromSeconds(mfr.Position / outputDevice.OutputWaveFormat.BitsPerSample / outputDevice.OutputWaveFormat.Channels * 8.0 / outputDevice.OutputWaveFormat.SampleRate);
                    Position = convered;
                }
            }
        }

        public void RePlay(Music music2play)
        {
            if (outputDevice == null)
            {
                outputDevice = new WaveOutEvent();
            }
            if (mfr == null)
            {
                mfr = new MediaFoundationReader(music2play.Url);
                outputDevice.Init(mfr);
                outputDevice.Volume = (float)Volume / 100;
            }
            else
            {
                if (music2play.Url != null)
                {
                    outputDevice.Stop();
                    mfr = new MediaFoundationReader(music2play.Url);
                    outputDevice.Init(mfr);
                    outputDevice.Volume = (float)Volume / 100;
                }
            }

            if (!musicNow.Equals(music2play))
            {
                musicNow = music2play;

                durationTime = music2play.DuartionTime;
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(DurationTimeDouble)));
                PropertyChanged(this, new PropertyChangedEventArgs(nameof(DurationTimeString)));
            }
            Play(true);
        }

        public void Play(bool re = false)
        {
            try
            {
                if (re)
                {
                    IsPlaying = true;
                    mfr.Position = TimeSpan.Zero.Ticks;
                    outputDevice.Play();
                }
                else
                {
                    if (!IsPlaying)
                    {
                        outputDevice.Play();
                        IsPlaying = true;
                        // ResEntry.musicInfo.DurationTime = outputDevice.GetPositionTimeSpan
                    }
                    else
                    {
                        outputDevice.Pause();
                        IsPlaying = false;
                        // ResEntry.musicInfo.DurationTime = player.NaturalDuration.TimeSpan;
                    }
                }
            }
            catch (InvalidOperationException)
            {
            }
        }

        public void SetPosition(double position)
        {
            if (IsPlaying)
            {
                long pos = (long)(position * 10) * outputDevice.OutputWaveFormat.BitsPerSample * outputDevice.OutputWaveFormat.Channels * outputDevice.OutputWaveFormat.SampleRate / 8 / 10;
                mfr.Position = pos;
                TimeSpan convered = TimeSpan.FromSeconds(mfr.Position / outputDevice.OutputWaveFormat.BitsPerSample / outputDevice.OutputWaveFormat.Channels * 8.0 / outputDevice.OutputWaveFormat.SampleRate);
                Position = convered;
            }
        }
    }
}