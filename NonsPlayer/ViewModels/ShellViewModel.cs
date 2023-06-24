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

    private Brush _userFace;
    private string _originalLyric = string.Empty;
    private string _tranLyric = string.Empty;
    private string _obOLrc = string.Empty;
    private string _obTLrc = string.Empty;
    private Lyric? currentLyric = null;
    private Lyric? nextLyric = null;

    public string TranLyric
    {
        get => _tranLyric;
        set
        {
            _tranLyric = value;
            _obTLrc = value;
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
            _obOLrc = value;
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
            if (MusicPlayerHelper.Player.MusicNow.Lyrics == null)
            {
                return time;
            }

            if (currentLyric == null && MusicPlayerHelper.Player.MusicNow.Lyrics != null)
            {
                currentLyric = new Lyric(MusicPlayerHelper.Player.MusicNow.Name,
                    MusicPlayerHelper.Player.MusicNow.ArtistsName, TimeSpan.Zero);
                OriginalLyric = currentLyric.OriginalLyric;
                TranLyric = currentLyric.TranLyric;
            }


            if (currentLyric != null && nextLyric != null && time >= currentLyric.Time && time < nextLyric.Time)
            {
                return time;
            }

            for (int i = 0; i < MusicPlayerHelper.Player.MusicNow.Lyrics.Count; i++)
            {
                var currentItem = MusicPlayerHelper.Player.MusicNow.Lyrics.Lrc[i];
                if (i + 1 == MusicPlayerHelper.Player.MusicNow.Lyrics.Count)
                {
                    return time;
                }

                var nextItem = MusicPlayerHelper.Player.MusicNow.Lyrics.Count > i
                    ? MusicPlayerHelper.Player.MusicNow.Lyrics.Lrc[i + 1]
                    : currentItem;
                if (time >= currentItem.Time && time < nextItem.Time)
                {
                    currentLyric = currentItem;
                    nextLyric = nextItem;
                    OriginalLyric = currentItem.OriginalLyric;
                    TranLyric = currentItem.TranLyric;
                    return time;
                }

                if (time <= MusicPlayerHelper.Player.MusicNow.Lyrics.Lrc[0].Time)
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
        NavigationService.Navigated += OnNavigated;

        MenuHomeOpenCommand = new RelayCommand(OnMenuHomeOpen);
        MenuExploreOpenCommand = new RelayCommand(OnMenuExploreOpen);
        MenuOwnOpenCommand = new RelayCommand(OnMenuOwnOpen);
        MenuSettingsCommand = new RelayCommand(OnMenuSettings);
        UserFace = new ImageBrush()
        {
            ImageSource =
                new BitmapImage(new Uri("ms-appdata:///Assets/UnKnowResource.png"))
        };
        MusicPlayerHelper.Player.PositionChangedHandle += LyricChanger;
        MusicPlayerHelper.Player.MusicChangedHandle += MusicChanged;

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

    public ICommand MenuOwnOpenCommand
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

    private void OnMenuOwnOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuSettings() => NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);

    #endregion 页面注册
}