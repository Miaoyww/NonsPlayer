using System.Windows.Forms;
using Windows.UI.Core;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class RecentlyPlayItemCard : UserControl
{
    public RecentlyPlayItemCard()
    {
        ViewModel = App.GetService<RecentlyPlayItemCardViewModel>();
        InitializeComponent();

        this.ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
    }

    public RecentlyPlayItemCardViewModel ViewModel { get; }

    public IMusic Music
    {
        set
        {
            for (var i = 0; i < value.Artists.Length; i++)
            {
                CheckArtists.Items.Add(new MenuFlyoutItem
                {
                    Text = value.Artists[i].Name,
                    Command = CheckArtistCommand,
                    Style = App.Current.Resources["CustomMenuFlyoutItem"] as Style,
                    CommandParameter = value.Artists[i]
                });
            }

            ViewModel.Init(value);
        }
    }

    [RelayCommand]
    public void CheckArtist(IArtist artist)
    {
        ServiceHelper.NavigationService.NavigateTo(typeof(ArtistViewModel)?.FullName, artist);
    }

    private void BodyBorder_OnTapped(object sender, TappedRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(ViewModel.Music);
    }
}