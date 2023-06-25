using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsApi;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class PersonalCenterPage : Page
{
    public PersonalCenterViewModel ViewModel
    {
        get;
    }

    public PersonalCenterPage()
    {
        ViewModel = App.GetService<PersonalCenterViewModel>();
        InitializeComponent();
    }


}
