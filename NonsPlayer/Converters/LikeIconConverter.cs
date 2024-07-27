using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using NonsPlayer.Helpers;

namespace NonsPlayer.Converters;

public class LikeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value) return App.Current.Resources["IconActiveColor"] as SolidColorBrush;

        return App.Current.Resources["IconCommonColor"] as SolidColorBrush;
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}