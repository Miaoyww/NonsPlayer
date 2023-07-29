using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Services;
using NonsPlayer.Models;

namespace NonsPlayer.ViewModels;

public class PersonalCenterViewModel : ObservableRecipient
{
    public AccountState AccountState => AccountState.Instance;
    public INavigationService NavigationService
    {
        get;
    }

    public PersonalCenterViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public void PersonalCenterPage_OnLoaded(object sender, RoutedEventArgs e)
    {
        if (!Account.Instance.IsLoggedIn)
        {
            NavigationService.NavigateTo(typeof(LoginViewModel).FullName!);
        }
    }
}