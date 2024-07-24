using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public class PersonalCenterViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public PersonalCenterViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public INavigationService NavigationService { get; }

    public void PersonalCenterPage_OnLoaded(object sender, RoutedEventArgs e)
    {
        //TODO: 本地账号设置
        // if (!Account.Instance.IsLoggedIn) NavigationService.NavigateTo(typeof(LoginViewModel).FullName!);
    }
}