using System.Collections.ObjectModel;
using System.Drawing.Text;
using System.Text;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Nons;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using static NonsPlayer.Core.Services.ControlFactory;

namespace NonsPlayer.ViewModels;

public partial class HomeViewModel : ObservableRecipient
{
    public INavigationService NavigationService;
    [ObservableProperty] public string installedFonts;
    [ObservableProperty] private ObservableCollection<IPlaylist>? recommendedPlaylist = new();

    public HomeViewModel(INavigationService navigationService)
    {
        NavigationService = navigationService;
    }

    [RelayCommand]
    public void Test()
    {
        var ifc = new InstalledFontCollection();
        var t = ifc.Families;
        StringBuilder sb = new();
        foreach (var item in t)
        {
            sb.Append(item.Name + " /");
        }
    }

    public async void HomePage_OnLoaded(object sender, RoutedEventArgs e)
    {
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Common);
        List<IPlaylist> recommendedPlaylist = new();
        foreach (var item in adapters)
        {
            recommendedPlaylist.AddRange(await item.Common.GetRecommendedPlaylistAsync(null, 6));
        }
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            foreach (var item in recommendedPlaylist)
            {
                RecommendedPlaylist.Add(item);
            }
        });
    }
}