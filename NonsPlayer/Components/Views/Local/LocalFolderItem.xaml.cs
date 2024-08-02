using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.WinUI.UI.Controls;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Components.Models;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System.Collections.ObjectModel;
using Windows.UI.Core;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views;

[INotifyPropertyChanged]
public sealed partial class LocalFolderItem : UserControl
{
    public LocalFolderItem()
    {
        ViewModel = App.GetService<LocalFolderItemViewModel>();
        ProtectedCursor = InputCursor.CreateFromCoreCursor(new CoreCursor(CoreCursorType.Hand, 0));
        InitializeComponent();
    }


    public LocalFolderItemViewModel ViewModel { get; }

    public LocalFolderModel Model
    {
        set
        {
            ViewModel.Name = value.Name;
            ViewModel.Index = value.Index;
            ViewModel.Path = value.Path;
        }
    }

    private void LocalFolderItem_OnDoubleTapped(object sender, DoubleTappedRoutedEventArgs e)
    {
        ViewModel.OpenFolder();
    }
}