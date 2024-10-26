using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using Windows.UI.Core;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LocalArtistItem : UserControl
{
    [ObservableProperty] private LocalArtist artist;
    [ObservableProperty] private string index;

    public LocalArtistItem()
    {
        ViewModel = App.GetService<LocalArtistItemViewModel>();
        ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
        InitializeComponent();
    }

    partial void OnIndexChanged(string index)
    {
        ViewModel.Index = index;
    }

    partial void OnArtistChanged(LocalArtist value)
    {
        if (value != null)
        {
            ViewModel.Artist = value;
            ViewModel.Name = value.Name;
            ViewModel.Count = string.Format("SongCount".GetLocalized(), value.Songs.Count);
            foreach (var music in value.Songs)
            {
                DispatcherQueue.TryEnqueue(async () =>
                {
                    ViewModel.Cover = await ImageHelpers.GetImageBrushAsyncFromBytes(((LocalMusic)music).Cover);
                });


                break;
            }
        }
    }

    public LocalArtistItemViewModel ViewModel { get; }

    public void OpenArtist(object sender, DoubleTappedRoutedEventArgs e)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

}