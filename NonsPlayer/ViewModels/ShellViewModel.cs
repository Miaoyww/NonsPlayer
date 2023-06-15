using System.ComponentModel;
using System.Diagnostics;
using System.Windows.Input;
using Windows.System;
using Windows.UI;
using ABI.Windows.System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Framework.Model;
using NonsPlayer.Helpers;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Framework.Player;
using NonsPlayer.Framework.Resources;

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
            for (int i = 0; i < MusicPlayerHelper.Player.MusicNow.Lyrics.Count; i++)
            {
                var item = MusicPlayerHelper.Player.MusicNow.Lyrics.Lrc[i];
                if (time.TotalMilliseconds >= item.Time.TotalMilliseconds)
                {
                    if (i + 1 < MusicPlayerHelper.Player.MusicNow.Lyrics.Count &&
                        time.TotalMilliseconds <
                        MusicPlayerHelper.Player.MusicNow.Lyrics.Lrc[i + 1].Time.TotalMilliseconds)
                    {
                        if (!_obOLrc.Equals(item.OriginalLyric))
                        {
                            OriginalLyric = item.OriginalLyric;
                        }

                        if (!_obTLrc.Equals(item.TranLyric))
                        {
                            TranLyric = item.TranLyric;
                        }
                    }
                }
            }
        }
        catch (Exception e)
        {

        }
        

        return time;
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
        MusicPlayer.PositionChangerHandle += LyricChanger;
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