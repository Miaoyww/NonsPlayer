using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class ExplorePage : Page
{
    public ExploreViewModel ViewModel
    {
        get;
    }

    public ExplorePage()
    {
        ViewModel = App.GetService<ExploreViewModel>();
        InitializeComponent();
    }
}