using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using Windows.UI.Core;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class BestArtistCard : UserControl
{
    [ObservableProperty] private ImageSource cover;
    [ObservableProperty] private string? id;
    [ObservableProperty] private string name;
    [ObservableProperty] private IArtist currentArtist;


    public BestArtistCard()
    {
        ViewModel = App.GetService<BestArtistCardViewModel>();
        ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
        InitializeComponent();
    }

    public IArtist Artist
    {
        set
        {
            if (value != null)
            {
                CurrentArtist = value;
            }
        }
    }

    public BestArtistCardViewModel ViewModel { get; }

    async partial void OnCurrentArtistChanged(IArtist value)
    {
        Cover = (await CacheHelper.GetImageBrushAsync(value.CacheAvatarId, value.AvatarUrl)).ImageSource;
        Name = value.Name;
        Id = value.Id;
    }


    partial void OnCoverChanged(ImageSource value)
    {
        BeginAnimation();
        AvatarAnimation.Completed += (sender, o) => { BeginAnimation(); };
    }


    private async void BeginAnimation()
    {
        AvatarAnimation.Children[0].SetValue(DoubleAnimation.FromProperty, AvatarTransform.Y);
        AvatarAnimation.Children[0].SetValue(DoubleAnimation.ToProperty, AvatarTransform.Y <= -300 ? 0 : -300);
        AvatarAnimation.Begin();
    }

    private async void OnBestMusicResultChanged(IMusic value)
    {
        Id = value.Artists[0].Id;
        Name = value.Artists[0].Name;
        // value.Artists[0] = await ArtistAdapters.CreateById(value.Artists[0].Id);
        if (value.Artists[0].AvatarUrl == null)
        {
            value.Artists[0] = await AdapterService.Instance.GetAdapter("ncm").Artist
                .GetArtistAsyncById(value.Artists[0].Id);
        }

        var coverTemp = await CacheHelper.GetImageBrushAsync(value.Artists[0].CacheMiddleAvatarId,
            value.Artists[0].MiddleAvatarUrl);
        Avater.DispatcherQueue.TryEnqueue(() => { Cover = coverTemp.ImageSource; });
    }

    private void UIElement_OnTapped(object sender, TappedRoutedEventArgs e)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, CurrentArtist);
    }
}