using System.ComponentModel;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Account;

namespace NonsPlayer.Models;

public class AccountState : INotifyPropertyChanged
{
    public static AccountState Instance
    {
        get;
    } = new();

    private string _uid;
    private string _name;
    private string _faceUrl;
    private ImageBrush _face;

    public string Uid
    {
        get => _uid;
        set
        {
            _uid = value;
            OnPropertyChanged();
        }
    }

    public string Name
    {
        get => _name;
        set
        {
            _name = value;
            OnPropertyChanged();
        }
    }

    public string FaceUrl
    {
        get => _faceUrl;
        set
        {
            _faceUrl = value;
            OnPropertyChanged();
            OnPropertyChanged(nameof(Face));
        }
    }

    public ImageBrush Face
    {
        get
        {
            if (_face == null)
            {
                _face = new ImageBrush
                {
                    ImageSource = new BitmapImage(new Uri(FaceUrl))
                };
            }

            return _face;
        }
        set
        {
            _face = value;
            OnPropertyChanged();
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}