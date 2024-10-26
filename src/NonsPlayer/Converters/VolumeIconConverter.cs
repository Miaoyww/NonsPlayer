using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Converters;

public class VolumeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return (double)value switch
        {
            var n when n <= 10 => "\ue992",
            var n when n is <= 50 and > 10 => "\ue993",
            var n when n is <= 100 and > 50 => "\ue994",
            _ => "\ue992"
        };
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}