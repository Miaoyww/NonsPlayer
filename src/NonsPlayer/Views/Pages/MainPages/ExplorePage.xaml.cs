using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class ExplorePage : Page
{
    public ExplorePage()
    {
        ViewModel = App.GetService<ExploreViewModel>();
        InitializeComponent();
    }

    public ExploreViewModel ViewModel { get; }
}