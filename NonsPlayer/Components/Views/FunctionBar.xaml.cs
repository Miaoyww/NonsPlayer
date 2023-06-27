using Microsoft.UI.Xaml.Controls;
using NonsPlayer.Components.ViewModels;

namespace NonsPlayer.Components.Views;
public sealed partial class FunctionBar : UserControl
{
    public FunctionBarViewModel ViewModel
    {
        get;
    }
    public FunctionBar()
    {
        ViewModel = App.GetService<FunctionBarViewModel>();
        InitializeComponent();
    }
}
