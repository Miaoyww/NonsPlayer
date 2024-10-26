using Windows.Storage.Streams;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;

namespace NonsPlayer.Helpers;

public static class RequestHelper
{
    public static ImageBrush GetImageBrushAsync(string url)
    {
        return new ImageBrush
        {
            ImageSource = new BitmapImage(new Uri(url))
        };
    }
}