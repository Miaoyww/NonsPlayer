using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Helpers;

namespace NonsPlayer.ViewModels;

[INotifyPropertyChanged]
public partial class AccountStateModel
{
    [ObservableProperty] private ImageBrush face;
    [ObservableProperty] private string faceUrl;
    [ObservableProperty] private string name;

    [ObservableProperty] private string uid;

    public static AccountStateModel Instance { get; } = new();

    partial void OnFaceUrlChanged(string value)
    {
        ServiceHelper.DispatcherQueue.TryEnqueue(() =>
        {
            Face = new ImageBrush
            {
                ImageSource = new BitmapImage(new Uri(value))
            };
        });
        OnPropertyChanged(nameof(Face));
    }
}