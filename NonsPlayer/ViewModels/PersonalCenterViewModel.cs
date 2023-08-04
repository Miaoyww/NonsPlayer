﻿using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core;
using NonsPlayer.Core.Account;
using NonsPlayer.Core.Api;
using NonsPlayer.Core.Services;
using NonsPlayer.Models;

namespace NonsPlayer.ViewModels;

public class PersonalCenterViewModel : ObservableRecipient, INotifyPropertyChanged
{
    public AccountState AccountState => AccountState.Instance;

    public string Greeting
    {
        get
        {
            var hour = DateTime.Now.Hour;
            switch (hour)
            {
                case int n when n >= 0 && n < 6:
                    return "凌晨好!";
                case int n when n >= 6 && n < 12:
                    return "早上好!";
                case int n when n >= 12 && n < 14:
                    return "中午好!";
                case int n when n >= 14 && n < 18:
                    return "下午好!";
                default:
                    return "晚上好!";
            }
        }
    }

    public INavigationService NavigationService { get; }

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