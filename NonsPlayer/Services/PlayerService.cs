using System.Timers;
using NAudio.Wave;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using NonsPlayer.Data;
using Timer = System.Timers.Timer;

namespace NonsPlayer.Services;

public class PlayerService
{
    public static PlayerService Instance
    {
        get;
    } = new PlayerService();

    private MediaFoundationReader _mfr;
    public WaveOutEvent OutputDevice;

    /// <summary>
    /// 初始化播放器
    /// </summary>
    public PlayerService()
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
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            if (OutputDevice != null)
            {
                if (OutputDevice.PlaybackState == PlaybackState.Playing)
                {
                    MusicState.Instance.Position = TimeSpan.FromSeconds(_mfr.Position /
                        OutputDevice.OutputWaveFormat.BitsPerSample /
                        OutputDevice.OutputWaveFormat.Channels * 8.0 / OutputDevice.OutputWaveFormat.SampleRate);
                    MusicState.Instance.PositionChangedHandle(MusicState.Instance.Position);
                }

                if (MusicState.Instance.Position == MusicState.Instance.CurrentMusic.DuartionTime)
                {
                    MusicState.Instance.OnNextMusic();
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

        MusicState.Instance.CurrentMusic = music2play;
        await MusicState.Instance.CurrentMusic.GetLyric();
        await MusicState.Instance.CurrentMusic.GetFileInfo();
        if (_mfr == null)
        {
            _mfr = new MediaFoundationReader(music2play.Url);
            OutputDevice.Init(_mfr);
            OutputDevice.Volume = (float)MusicState.Instance.Volume / 100;
        }
        else
        {
            if (music2play.Url != null)
            {
                OutputDevice.Stop();
                _mfr = new MediaFoundationReader(music2play.Url);
                OutputDevice.Init(_mfr);
                OutputDevice.Volume = (float)MusicState.Instance.Volume / 100;
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
        if (MusicState.Instance.CurrentMusic == null)
        {
            return;
        }

        try
        {
            if (re)
            {
                MusicState.Instance.IsPlaying = true;
                _mfr.Position = TimeSpan.Zero.Ticks;
                OutputDevice.Play();
            }
            else
            {
                if (!MusicState.Instance.IsPlaying)
                {
                    OutputDevice.Play();
                    MusicState.Instance.IsPlaying = true;
                }
                else
                {
                    OutputDevice.Pause();
                    MusicState.Instance.IsPlaying = false;
                }
            }
        }
        catch (InvalidOperationException)
        {
        }
    }
}

public class PlayQueueService
    {
        public static PlayQueueService Instance
        {
            get;
        } = new();

        private Music _currentMusic;
        private List<Music> _randomMusicList = new();

        public List<Music> MusicList
        {
            get;
            set;
        }

        public Music CurrentMusic
        {
            get => _currentMusic;
            set
            {
                if (value == _currentMusic)
                {
                    return;
                }

                _currentMusic = value;
                OnCurrentMusicChanged();
            }
        }

        public enum PlayModeEnum
        {
            Sequential = 0, //顺序播放
            Random = 1, //随机播放
            SingleLoop = 2, //单曲循环
            ListLoop = 3, //列表循环
        }

        /// <summary>
        /// 模式选择器
        /// </summary>
        public PlayModeEnum PlayMode
        {
            get;
            set;
        }

        public PlayQueueService()
        {
            MusicList = new();
            PlayMode = PlayModeEnum.ListLoop;
        }

        // 当CurrentMusic改变时，将触发PlayerService的NewPlay方法
        public void OnCurrentMusicChanged()
        {
            PlayerService.Instance.NewPlay(CurrentMusic);
        }

        /// <summary>
        /// 改变模式
        /// </summary>
        public void ChangePlayMode()
        {
            PlayMode = PlayMode switch
            {
                PlayModeEnum.Sequential => PlayModeEnum.Random,
                PlayModeEnum.Random => PlayModeEnum.SingleLoop,
                PlayModeEnum.SingleLoop => PlayModeEnum.ListLoop,
                PlayModeEnum.ListLoop => PlayModeEnum.Sequential,
                _ => PlayModeEnum.Sequential
            };
            // 如果模式是随机播放，调用GetRandomList方法
            if (PlayMode is PlayModeEnum.Random)
            {
                _randomMusicList = GetRandomList(CurrentMusic);
            }
        }

        /// <summary>
        /// 添加歌单到播放列表
        /// </summary>
        /// <param name="musicList">歌曲</param>
        public void AddMusicList(List<Music> musicList)
        {
            Clear();
            MusicList.AddRange(musicList);
            Play(musicList[0]);
        }

        /// <summary>
        /// 添加歌曲到播放列表
        /// </summary>
        /// <param name="music">待添加的歌曲</param>
        public void AddMusic(Music music)
        {
            // 若播放列表为空，那么直接添加到播放列表中
            if (MusicList.Count == 0)
            {
                MusicList.Add(music);
                CurrentMusic = music;
                return;
            }

            // 如果播放列表中已存在该歌曲，那么直接播放该歌曲
            if (MusicList.Contains(music))
            {
                CurrentMusic = music;
                return;
            }

            // 如果播放列表不为空，那么就在当前播放歌曲的后面插入
            MusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
            if (PlayMode is PlayModeEnum.Random)
            {
                // 直接在随机播放列表的当前播放音乐后插入
                _randomMusicList.Insert(MusicList.IndexOf(CurrentMusic) + 1, music);
            }

            CurrentMusic = music;
        }

        /// <summary>
        /// 删除指定歌曲
        /// </summary>
        /// <param name="music">待删除的歌曲</param>
        public void RemoveMusic(Music music)
        {
            MusicList.Remove(music);
            if (PlayMode is PlayModeEnum.Random)
            {
                _randomMusicList.Remove(music);
            }
        }

        /// <summary>
        /// 清空播放列表
        /// </summary>
        public void Clear()
        {
            MusicList.Clear();
            _randomMusicList.Clear();
        }

        /// <summary>
        /// 播放歌曲
        /// </summary>
        public void Play(Music music)
        {
            // 若当前音乐正在播放，则触发Play，正在播放则暂停，否则播放
            if (music == CurrentMusic)
            {
                PlayerService.Instance.Play();
                return;
            }

            CurrentMusic = music;
        }

        /// <summary>
        /// 播放下一首歌曲
        /// </summary>
        public void PlayNext()
        {
            // 若当前播放模式为随机播放，则获取随机播放列表
            var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;

            // 如果是单曲循环，那么就播放当前歌曲
            if (PlayMode is PlayModeEnum.SingleLoop)
            {
                Play(CurrentMusic);
                return;
            }

            // 如果是顺序播放，那么到了最后一首歌就停止播放
            var index = list.IndexOf(CurrentMusic) + 1;
            if (PlayMode is PlayModeEnum.Sequential)
            {
                if (index >= list.Count)
                {
                    return;
                }

                Play(list[index]);
                return;
            }

            // 如果是列表循环，那么到了最后一首歌就播放第一首歌
            if (PlayMode is PlayModeEnum.ListLoop)
            {
                if (list.Count == 0)
                {
                    return;
                }
                index = index >= list.Count ? 0 : index;
                Play(list[index]);
            }
        }

        /// <summary>
        /// 播放上一首歌曲
        /// </summary>
        public void PlayPrevious()
        {
            // 若当前播放模式为随机播放，则获取随机播放列表
            var list = PlayMode is PlayModeEnum.Random ? _randomMusicList : MusicList;
            // 获取上一首歌曲
            var index = list.IndexOf(CurrentMusic) - 1;
            if (index < 0)
            {
                index = list.Count - 1;
            }

            Play(list[index]);
        }

        /// <summary>
        /// 获取随机播放列表
        /// </summary>
        /// <param name="music">当前播放的歌曲</param>
        private List<Music> GetRandomList(Music music)
        {
            var random = new Random();
            var list = MusicList.ToList();
            list.RemoveAt(0);
            list = list.OrderBy(x => random.Next()).ToList();
            list.Insert(0, music);
            return list;
        }
    }