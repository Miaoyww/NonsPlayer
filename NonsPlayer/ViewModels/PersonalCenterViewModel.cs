using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Nons.Account;

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
                    return "夜深了, ";
                case int n when n >= 6 && n < 12:
                    return "Good Morning~";
                case int n when n >= 12 && n < 14:
                    return "中午好!";
                case int n when n >= 14 && n < 18:
                    return "来杯下午茶吧, ";
                default:
                    return "日落归去, 晚好,";
            }
        }
    }

    public INavigationService NavigationService { get; }

    public void PersonalCenterPage_OnLoaded(object sender, RoutedEventArgs e)
    {
        if (!Account.Instance.IsLoggedIn) NavigationService.NavigateTo(typeof(LoginViewModel).FullName!);
    }
}