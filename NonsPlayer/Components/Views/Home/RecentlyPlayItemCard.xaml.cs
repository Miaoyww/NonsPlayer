using System.Windows.Forms;
using Windows.UI.Core;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Input;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Nons.Player;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;

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
        set => ViewModel.Init(value);
    }

    private void BodyBorder_OnPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        PlayQueue.Instance.Play(ViewModel.Music);
    }
}