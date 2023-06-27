using System.Timers;
using NAudio.Wave;
using NonsPlayer.Framework.Model;
using NonsPlayer.Framework.Resources;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Framework.Player;

public class MusicPlayer
{
    public static MusicPlayer Instance
    {
        get;
    } = new MusicPlayer();

    private MediaFoundationReader _mfr;
    public WaveOutEvent OutputDevice;

    /// <summary>
    /// 初始化播放器
    /// </summary>
    public MusicPlayer()
    {
        OutputDevice = new WaveOutEvent();
        var timer = new Timer();
        timer.Interval = 20;
        timer.Elapsed += GetCurrentInfo;
        timer.Start();
    }

    /// <summary>
    /// 用于获取播放器当前信息
    /// 包括当前Position
    /// </summary>
    private void GetCurrentInfo(object? sender, ElapsedEventArgs e)
    {
        ServiceEntry.DispatcherQueue.TryEnqueue(() =>
        {
            if (OutputDevice != null)
            {
                if (OutputDevice.PlaybackState == PlaybackState.Playing)
                {
                    GlobalMusicState.Instance.Position = TimeSpan.FromSeconds(_mfr.Position /
                        OutputDevice.OutputWaveFormat.BitsPerSample /
                        OutputDevice.OutputWaveFormat.Channels * 8.0 / OutputDevice.OutputWaveFormat.SampleRate);
                    GlobalMusicState.Instance.PositionChangedHandle(GlobalMusicState.Instance.Position);
                }

                if (GlobalMusicState.Instance.Position == GlobalMusicState.Instance.CurrentMusic.DuartionTime)
                {
                    GlobalMusicState.Instance.OnNextMusic();
                }
            }
        });
    }

    /// <summary>
    /// 播放一个新的音乐
    /// </summary>
    /// <param name="music2play">即将播放的音乐</param>
    public async void NewPlay(Music music2play)
    {
        if (OutputDevice == null)
        {
            OutputDevice = new WaveOutEvent();
        }

        GlobalMusicState.Instance.CurrentMusic = music2play;
        await GlobalMusicState.Instance.CurrentMusic.GetLric();
        await GlobalMusicState.Instance.CurrentMusic.GetFileInfo();
        if (_mfr == null)
        {
            _mfr = new MediaFoundationReader(music2play.Url);
            OutputDevice.Init(_mfr);
            OutputDevice.Volume = (float)GlobalMusicState.Instance.Volume / 100;
        }
        else
        {
            if (music2play.Url != null)
            {
                OutputDevice.Stop();
                _mfr = new MediaFoundationReader(music2play.Url);
                OutputDevice.Init(_mfr);
                OutputDevice.Volume = (float)GlobalMusicState.Instance.Volume / 100;
            }
        }

        Play(true);
    }

    /// <summary>
    /// 播放音乐
    /// </summary>
    /// <param name="re">是否从头播放</param>
    public void Play(bool re = false)
    {
        if (GlobalMusicState.Instance.CurrentMusic == null)
        {
            return;
        }

        try
        {
            if (re)
            {
                GlobalMusicState.Instance.IsPlaying = true;
                _mfr.Position = TimeSpan.Zero.Ticks;
                OutputDevice.Play();
            }
            else
            {
                if (!GlobalMusicState.Instance.IsPlaying)
                {
                    OutputDevice.Play();
                    GlobalMusicState.Instance.IsPlaying = true;
                }
                else
                {
                    OutputDevice.Pause();
                    GlobalMusicState.Instance.IsPlaying = false;
                }
            }
        }
        catch (InvalidOperationException)
        {
        }
    }
}