using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

public class PersonalCenterViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public PersonalCenterViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    public AccountStateModel AccountStateModel => AccountStateModel.Instance;

    public string Greeting
    {
        //TODO: 应用一言 https://developer.hitokoto.cn/
        get
        {
            var hour = DateTime.Now.Hour;
            switch (hour)
            {
                case int n when n >= 0 && n < 6:
                    return "Greetings_MidNight".GetLocalized();
                case int n when n >= 6 && n < 12:
                    return "Greetings_Morning".GetLocalized();
                case int n when n >= 12 && n < 14:
                    return "Greetings_Noon".GetLocalized();
                case int n when n >= 14 && n < 18:
                    return "Greetings_Afternoon".GetLocalized();
                default:
                    return "Greetings_Night".GetLocalized();
            }
        }
    }

    public INavigationService NavigationService { get; }

    public void PersonalCenterPage_OnLoaded(object sender, RoutedEventArgs e)
    {
        if (!Account.Instance.IsLoggedIn) NavigationService.NavigateTo(typeof(LoginViewModel).FullName!);
    }
}