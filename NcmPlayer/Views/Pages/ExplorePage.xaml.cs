using Microsoft.UI.Xaml.Controls;

using NcmPlayer.ViewModels;

namespace NcmPlayer.Views;

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
