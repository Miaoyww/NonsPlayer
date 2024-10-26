using System.Diagnostics;
using System.Drawing.Imaging;
using System.Reflection;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.ViewModels;

public partial class TestViewModel : ObservableObject
{
    [RelayCommand]
    public async Task Test(string data)
    {
        Assembly assembly = Assembly.LoadFrom(data);
        
        Type[] types = assembly.GetTypes();
        foreach (Type type in types)
        {
            if (typeof(IMusicAdapter).IsAssignableFrom(type))
            {
                var musicProvider = (IMusicAdapter)Activator.CreateInstance(type);
                if (musicProvider != null)
                {
                    var music = await musicProvider.GetMusicAsync(2026565329L);
                    Debug.WriteLine(music.Name);
                    Debug.WriteLine(music.Id);
                }
        
                break;
            }
        }
    }
}