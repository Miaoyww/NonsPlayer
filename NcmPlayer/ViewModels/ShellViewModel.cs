using System.ComponentModel;
using System.Windows.Input;
using Windows.UI;
using ABI.Windows.System;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Navigation;
using NcmPlayer.Contracts.Services;
using NcmPlayer.Framework.Model;
using NcmPlayer.Helpers;

namespace NcmPlayer.ViewModels;

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

    public Brush UserFace
    {
        set
        {
            _userFace = value;
            OnPropertyChanged(nameof(UserFace));
        }
        get => _userFace;
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
    }

    private void OnNavigated(object sender, NavigationEventArgs e) => IsBackEnabled = NavigationService.CanGoBack;

    private void OnMenuHomeOpen() => NavigationService.NavigateTo(typeof(HomeViewModel).FullName!);

    private void OnMenuExploreOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuOwnOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuSettings() => NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);

    #endregion 页面注册
}