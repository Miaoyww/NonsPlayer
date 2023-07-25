using System.ComponentModel;
using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using Windows.System;

namespace NonsPlayer.ViewModels;

public class ShellViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public AccountService Account => AccountService.Instance;
    private bool _isBackEnabled;

    public bool IsBackEnabled
    {
        get => _isBackEnabled;
        set => SetProperty(ref _isBackEnabled, value);
    }


    public ShellViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
        ServiceHelper.NavigationService = navigationService;
        NavigationService.Navigated += OnNavigated;

        MenuHomeOpenCommand = new RelayCommand(OnMenuHomeOpen);
        MenuExploreOpenCommand = new RelayCommand(OnMenuExploreOpen);
        MenuPersonalCenterMenuOwnOpenCommand = new RelayCommand(OnMenuPersonalCenterOpen);
        MenuSettingsCommand = new RelayCommand(OnMenuSettings);
        AccountService.Instance.LoginByReg();
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
            PlaylistHelper.OpenMusicListDetail(playlistId, App.GetService<INavigationService>());
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

    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}