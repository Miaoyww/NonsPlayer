using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Converters;

public class ThumbConverter : DependencyObject, IValueConverter
{
    // Using a DependencyProperty as the backing store for SecondValue.  This enables animation, styling, binding, etc...
    public static readonly DependencyProperty SecondValueProperty =
        DependencyProperty.Register("SecondValue", typeof(double), typeof(ThumbConverter),
            new PropertyMetadata(0d));

    public double SecondValue
    {
        get => (double)GetValue(SecondValueProperty);
        set => SetValue(SecondValueProperty, value);
    }


    public object Convert(object value, Type targetType, object parameter, string language)
    {
        // assuming you want to display precentages

        return TimeSpan.FromSeconds(double.Parse(value.ToString())).ToString(@"hh\:mm\:ss");
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}