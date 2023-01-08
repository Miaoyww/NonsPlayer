using System.Windows.Input;

using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Navigation;

using NcmPlayer.Contracts.Services;
using NcmPlayer.Framework.Model;

namespace NcmPlayer.ViewModels;

public class ShellViewModel : ObservableRecipient
{
    private bool _isBackEnabled;

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

    public bool IsBackEnabled
    {
        get => _isBackEnabled;
        set => SetProperty(ref _isBackEnabled, value);
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
    }

    private void OnNavigated(object sender, NavigationEventArgs e) => IsBackEnabled = NavigationService.CanGoBack;

    private void OnMenuHomeOpen() => NavigationService.NavigateTo(typeof(HomeViewModel).FullName!);

    private void OnMenuExploreOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuOwnOpen() => NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);

    private void OnMenuSettings() => NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);

    #endregion 页面注册

    public Music? CurrentMusic;
}