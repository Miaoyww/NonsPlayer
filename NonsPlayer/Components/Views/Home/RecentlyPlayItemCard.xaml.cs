using System.Windows.Forms;
using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Music;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class RecentlyPlayItemCard : UserControl
{
    public RecentlyPlayItemCard()
    {
        ViewModel = App.GetService<RecentlyPlayItemCardViewModel>();
        InitializeComponent();
    }

    public RecentlyPlayItemCardViewModel ViewModel { get; }

    public IMusic Music
    {
        set => ViewModel.Init(value);
    }

    public string Url
    {
        set => ViewModel.Init(value);
    }
}