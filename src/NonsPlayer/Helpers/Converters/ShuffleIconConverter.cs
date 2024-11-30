using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Helpers.Converters;

public class ShuffleIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var isShuffle = (bool)value;
        if (isShuffle) return Visibility.Visible;

        return Visibility.Collapsed;
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}