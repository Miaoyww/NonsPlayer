using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Models;
using NonsPlayer.Resources;

namespace NonsPlayer.Components.ViewModels
{
    public class LyricBoxViewModel : INotifyPropertyChanged
    {
        private string _originalLyric = string.Empty;
        private string _tranLyric = string.Empty;

        private Lyric? _currentLyric = null;
        private Lyric? _nextLyric = null;

        public string TranLyric
        {
            get => _tranLyric;
            set
            {
                _tranLyric = value;
                OnPropertyChanged(nameof(TranLyric));
                OnPropertyChanged(nameof(TransVisibility));
            }
        }

        public string OriginalLyric
        {
            get => _originalLyric;
            set
            {
                _originalLyric = value;
                OnPropertyChanged(nameof(OriginalLyric));
            }
        }

        public Visibility TransVisibility
        {
            get
            {
                return TranLyric == "" ? Visibility.Collapsed : Visibility.Visible;
            }
        }


        private Music MusicChanged(Music currentMusic)
        {
            _currentLyric = null;
            _nextLyric = null;
            return currentMusic;
        }

        private TimeSpan LyricChanger(TimeSpan time)
        {
            try
            {
                if (GlobalMusicState.Instance.CurrentMusic.Lyrics == null)
                {
                    return time;
                }

                if (_currentLyric == null && GlobalMusicState.Instance.CurrentMusic.Lyrics != null)
                {
                    _currentLyric = new Lyric(GlobalMusicState.Instance.CurrentMusic.Name,
                        GlobalMusicState.Instance.CurrentMusic.ArtistsName, TimeSpan.Zero);
                    OriginalLyric = _currentLyric.OriginalLyric;
                    TranLyric = _currentLyric.TranLyric;
                }


                if (_currentLyric != null && _nextLyric != null && time >= _currentLyric.Time && time < _nextLyric.Time)
                {
                    return time;
                }

                for (int i = 0; i < GlobalMusicState.Instance.CurrentMusic.Lyrics.Count; i++)
                {
                    var currentItem = GlobalMusicState.Instance.CurrentMusic.Lyrics.Lrc[i];
                    if (i + 1 == GlobalMusicState.Instance.CurrentMusic.Lyrics.Count)
                    {
                        return time;
                    }

                    var nextItem = GlobalMusicState.Instance.CurrentMusic.Lyrics.Count > i
                        ? GlobalMusicState.Instance.CurrentMusic.Lyrics.Lrc[i + 1]
                        : currentItem;
                    if (time >= currentItem.Time && time < nextItem.Time)
                    {
                        _currentLyric = currentItem;
                        _nextLyric = nextItem;
                        OriginalLyric = currentItem.OriginalLyric;
                        TranLyric = currentItem.TranLyric;
                        return time;
                    }

                    if (time <= GlobalMusicState.Instance.CurrentMusic.Lyrics.Lrc[0].Time)
                    {
                        return time;
                    }
                }
            }
            catch (Exception e)
            {
            }


            return time;
        }

        public void Init()
        {
            GlobalMusicState.Instance.PositionChangedHandle += LyricChanger;
            GlobalMusicState.Instance.MusicChangedHandle += MusicChanged;
            OriginalLyric = "暂未播放";
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}