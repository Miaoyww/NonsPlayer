using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Converters;

public class LikeIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if ((bool)value) return "\uEB52";

        return "\uEB51";
    }

    public object? ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return null;
    }
}