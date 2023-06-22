using System.Security.Cryptography.X509Certificates;
using CommunityToolkit.Mvvm.ComponentModel;
using System.ComponentModel;
using System.Numerics;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Composition;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Helpers;
using CommunityToolkit.Mvvm.Input;
using System.Windows.Input;

namespace NonsPlayer.Components.ViewModels;

public partial class PlaylistCardViewModel : ObservableRecipient, INotifyPropertyChanged
{
    private readonly INavigationService _navigationService;

    public PlaylistCardViewModel(INavigationService navigationService)
    {
        _navigationService = navigationService;
    }

    public new event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(string propertyName) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));

    private ImageBrush _cover;
    private string _title;
    private string _id;

    public ImageBrush Cover
    {
        get => _cover;
        set
        {
            _cover = value;
            OnPropertyChanged(nameof(Cover));
        }
    }

    public string Title
    {
        get => _title;
        set
        {
            _title = value;
            OnPropertyChanged(nameof(Title));
        }
    }

    public void Init(JObject item)
    {
        var picUrl = item["picUrl"] + "?param=200y200";
        var title = (string)item["name"];
        _id = (string)item["id"];
        Title = title;
        Cover = new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(picUrl))
        };
    }

    public void OpenMusicListDetail(object sender, PointerRoutedEventArgs e) =>
        Tools.OpenMusicListDetail(long.Parse(_id), _navigationService);
}