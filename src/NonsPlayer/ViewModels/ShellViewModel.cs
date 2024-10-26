using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Views.Pages;

namespace NonsPlayer.ViewModels;

public partial class ShellViewModel : ObservableRecipient
{
    public static INavigationService OutNavigationService;
    private bool _isBackEnabled;
    public ShellViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
        OutNavigationService = NavigationService;
        NavigationService.Navigated += OnNavigated;
    }

    public bool IsBackEnabled
    {
        get => _isBackEnabled;
        set => SetProperty(ref _isBackEnabled, value);
    }


    #region 接口实现

    public INavigationService NavigationService { get; }

    #endregion 接口实现

    public void SearchBox_Entered(AutoSuggestBox sender, AutoSuggestBoxSuggestionChosenEventArgs text)
    {
        if (text.Equals(string.Empty)) return;

        NavigationService.NavigateTo(typeof(SearchViewModel).FullName!, text);
    }

    public void SearchBox_Query(AutoSuggestBox sender, AutoSuggestBoxQuerySubmittedEventArgs args)
    {
        if (args.QueryText.Equals(string.Empty)) return;

        NavigationService.NavigateTo(typeof(SearchViewModel).FullName!, args.QueryText);
    }

    #region 页面注册

    private void OnNavigated(object sender, NavigationEventArgs e)
    {
        IsBackEnabled = NavigationService.CanGoBack;
    }

    [RelayCommand]
    private void OpenPage(string param)
    {
        switch (param)
        {
            case "home":
                NavigationService.NavigateTo(typeof(HomeViewModel).FullName!);
                break;
            case "explore":
                NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);
                break;
            case "own":
                NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName!);
                break;
            case "local":
                NavigationService.NavigateTo(typeof(LocalViewModel).FullName!);
                break;
            case "settings":
                NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);
                break;
            default:
                break;
        }
    }

    [RelayCommand]
    private void GoBack()
    {
        NavigationService.GoBack();
    }

    #endregion 页面注册
}

[INotifyPropertyChanged]
public partial class PlayQueueBarViewModel
{
    [ObservableProperty] private int count;
    public ObservableCollection<MusicModel> MusicItems = new();

    public PlayQueueBarViewModel()
    {
        PlayQueue.Instance.MusicAdded += OnMusicAdded;
        MusicItems.CollectionChanged += OnCollectionChanged;
        PlayQueue.Instance.CurrentQueueChanged += InstanceOnCurrentQueueChanged;
    }

    private void InstanceOnCurrentQueueChanged()
    {
        MusicItems.Clear();
        PlayQueue.Instance.MusicList.ForEach(item =>
        {
            MusicItems.Add(new MusicModel { Music = item });
        });
    }

    public void OnCollectionChanged(object? sender, EventArgs e)
    {
        Count = MusicItems.Count;
    }

    public void OnMusicAdded(IMusic value)
    {
        // MusicItems.Insert(PlayQueue.Instance.GetIndex(value), new MusicItem
        // {
        //     Music = value
        // });
    }
}