using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml;
using NonsPlayer.Core.Models;
using NonsPlayer.Data;

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
                if (MetaData.Instance.CurrentMusic.Lyrics == null)
                {
                    return time;
                }

                if (_currentLyric == null && MetaData.Instance.CurrentMusic.Lyrics != null)
                {
                    _currentLyric = new Lyric(MetaData.Instance.CurrentMusic.Name,
                        MetaData.Instance.CurrentMusic.ArtistsName, TimeSpan.Zero);
                    OriginalLyric = _currentLyric.OriginalLyric;
                    TranLyric = _currentLyric.TranLyric;
                }


                if (_currentLyric != null && _nextLyric != null && time >= _currentLyric.Time && time < _nextLyric.Time)
                {
                    return time;
                }

                for (int i = 0; i < MetaData.Instance.CurrentMusic.Lyrics.Count; i++)
                {
                    var currentItem = MetaData.Instance.CurrentMusic.Lyrics.Lrc[i];
                    if (i + 1 == MetaData.Instance.CurrentMusic.Lyrics.Count)
                    {
                        return time;
                    }

                    var nextItem = MetaData.Instance.CurrentMusic.Lyrics.Count > i
                        ? MetaData.Instance.CurrentMusic.Lyrics.Lrc[i + 1]
                        : currentItem;
                    if (time >= currentItem.Time && time < nextItem.Time)
                    {
                        _currentLyric = currentItem;
                        _nextLyric = nextItem;
                        OriginalLyric = currentItem.OriginalLyric;
                        TranLyric = currentItem.TranLyric;
                        return time;
                    }

                    if (time <= MetaData.Instance.CurrentMusic.Lyrics.Lrc[0].Time)
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
            MetaData.Instance.PositionChangedHandle += LyricChanger;
            MetaData.Instance.MusicChangedHandle += MusicChanged;
            OriginalLyric = "暂未播放";
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}