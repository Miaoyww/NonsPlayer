using System.ComponentModel;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework.Model;
using NonsPlayer.Framework.Player;
using NonsPlayer.Framework.Resources;
using NonsPlayer.Helpers;
using Windows.System;

namespace NonsPlayer.ViewModels;

public class ShellViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    private bool _isBackEnabled;

    public bool IsBackEnabled
    {
        get => _isBackEnabled;
        set => SetProperty(ref _isBackEnabled, value);
    }

    public GlobalMusicState GlobalMusicState => GlobalMusicState.Instance;
    private Brush _userFace;
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

    private Music MusicChanged(Music currentMusic)
    {
        _currentLyric = null;
        _nextLyric = null;
        return currentMusic;
    }

    public Brush UserFace
    {
        set
        {
            _userFace = value;
            OnPropertyChanged(nameof(UserFace));
        }
        get => _userFace;
    }


    public ShellViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
        ServiceEntry.NavigationService = navigationService;
        NavigationService.Navigated += OnNavigated;

        MenuHomeOpenCommand = new RelayCommand(OnMenuHomeOpen);
        MenuExploreOpenCommand = new RelayCommand(OnMenuExploreOpen);
        MenuPersonalCenterMenuOwnOpenCommand = new RelayCommand(OnMenuPersonalCenterOpen);
        MenuSettingsCommand = new RelayCommand(OnMenuSettings);
        UserFace = new ImageBrush()
        {
            ImageSource =
                new BitmapImage(new Uri("ms-appdata:///Assets/UnKnowResource.png"))
        };
        GlobalMusicState.Instance.PositionChangedHandle += LyricChanger;
        GlobalMusicState.Instance.MusicChangedHandle += MusicChanged;

        OriginalLyric = "暂未播放";
    }


    public void SearchBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter)
        {
            return;
        }

        if (((AutoSuggestBox)sender).Text == string.Empty)
        {
            return;
        }

        long playlistId;
        if (long.TryParse(((AutoSuggestBox)sender).Text, out playlistId))
        {
            Tools.OpenMusicListDetail(playlistId, App.GetService<INavigationService>());
        }
    }

    #region 接口实现

    public ICommand MenuHomeOpenCommand
    {
        get;
    }

    public ICommand MenuExploreOpenCommand
    {
        get;
    }

    public ICommand MenuPersonalCenterMenuOwnOpenCommand
    {
        get;
    }

    public ICommand MenuSettingsCommand
    {
        get;
    }


    public INavigationService NavigationService
    {
        get;
    }

    #endregion 接口实现

    #region 页面注册

    private void OnNavigated(object sender, NavigationEventArgs e) => IsBackEnabled = NavigationService.CanGoBack;

    private void OnMenuHomeOpen() => NavigationService.NavigateTo(typeof(HomeViewModel).FullName!);

    private void OnMenuExploreOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuPersonalCenterOpen() => NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName!);

    private void OnMenuSettings() => NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);

    #endregion 页面注册
}