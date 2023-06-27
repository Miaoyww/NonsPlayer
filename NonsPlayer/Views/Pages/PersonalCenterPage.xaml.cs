using Microsoft.UI.Xaml.Controls;
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
