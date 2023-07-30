using System.ComponentModel;
using System.Runtime.CompilerServices;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Account;

namespace NonsPlayer.Models;

[INotifyPropertyChanged]
public partial class AccountState
{
    public static AccountState Instance
    {
        get;
    } = new();

    [ObservableProperty] private string uid;
    [ObservableProperty] private string name;
    [ObservableProperty] private string faceUrl;
    [ObservableProperty] private ImageBrush face;

    partial void OnFaceUrlChanged(string value)
    {
        face = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(value))
        };
        OnPropertyChanged(nameof(Face));
    }
}