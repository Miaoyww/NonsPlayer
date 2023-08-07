using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;

namespace NonsPlayer.Models;

[INotifyPropertyChanged]
public partial class AccountState
{
    [ObservableProperty] private ImageBrush face;
    [ObservableProperty] private string faceUrl;
    [ObservableProperty] private string name;

    [ObservableProperty] private string uid;

    public static AccountState Instance { get; } = new();

    partial void OnFaceUrlChanged(string value)
    {
        face = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(value))
        };
        OnPropertyChanged(nameof(Face));
    }
}