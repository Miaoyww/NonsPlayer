using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Windows.Foundation;
using Windows.Foundation.Collections;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Views.Local;

[INotifyPropertyChanged]
public sealed partial class LocalMusicLibPage : Page
{
    public LocalMusicLibPage()
    {
        InitializeComponent();
        ViewModel = App.GetService<LocalMusicLibViewModel>();
        ArtistTextBlock.Text = "Artist".GetLocalized();
        AlbumTextBlock.Text = "Album".GetLocalized();
        MusicTextBlock.Text = "Music".GetLocalized();
    }

    public LocalMusicLibViewModel ViewModel { get; }
}