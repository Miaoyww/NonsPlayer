﻿using Microsoft.UI.Xaml.Data;

namespace NonsPlayer.Converters;

public class ByteLengthToStringConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var length = System.Convert.ToDouble(value);
        return length switch
        {
            >= 1 << 30 => $"{length / (1 << 30):F2} GB",
            >= 1 << 20 => $"{length / (1 << 20):F2} MB",
            _ => $"{length / (1 << 10):F2} KB"
        };
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}