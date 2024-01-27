using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Components.Models;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons.Account;
using NonsPlayer.Core.Nons.Player;
using NonsPlayer.Helpers;
using NonsPlayer.Services;

namespace NonsPlayer.ViewModels;

public partial class ShellViewModel : ObservableRecipient
{
    private bool _isBackEnabled;


    public ShellViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
        ServiceHelper.NavigationService = navigationService;
        NavigationService.Navigated += OnNavigated;
        AccountService.Instance.UpdateInfo();
        Account.Instance.LoginByReg();
    }

    public AccountStateModel AccountStateModel => AccountStateModel.Instance;

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
    private void OpenMenuHome()
    {
        NavigationService.NavigateTo(typeof(HomeViewModel).FullName!);
    }

    [RelayCommand]
    private void OpenMenuExplore()
    {
        NavigationService.NavigateTo(typeof(ExploreViewModel).FullName!);
    }

    [RelayCommand]
    private void OpenMenuPersonalCenter()
    {
        NavigationService.NavigateTo(typeof(PersonalCenterViewModel).FullName!);
    }

    [RelayCommand]
    private void OpenMenuSettings()
    {
        NavigationService.NavigateTo(typeof(SettingsViewModel).FullName!);
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
    public ObservableCollection<MusicItem> MusicItems = new();

    public PlayQueueBarViewModel()
    {
        PlayQueue.Instance.MusicAdded += OnMusicAdded;
        PlayQueue.Instance.PlaylistAdded += OnPlaylistAdded;
        MusicItems.CollectionChanged += OnCollectionChanged;
    }

    public void OnCollectionChanged(object? sender, EventArgs e)
    {
        Count = MusicItems.Count;
    }

    public void OnPlaylistAdded()
    {
        MusicItems.Clear();
        PlayQueue.Instance.MusicList.ForEach(item =>
        {
            MusicItems.Add(new MusicItem
            {
                Music = item
            });
        });
    }

    public void OnMusicAdded(Music value)
    {
        MusicItems.Insert(PlayQueue.Instance.GetIndex(value), new MusicItem
        {
            Music = value
        });
    }

    public void DoubleClick(object sender, DoubleTappedRoutedEventArgs e)
    {
        var listView = sender as ListView;
        if (listView.SelectedItem is MusicItem item) PlayQueue.Instance.Play(item.Music);
    }
}