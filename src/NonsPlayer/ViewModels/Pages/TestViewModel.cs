using System.Diagnostics;
using System.Drawing.Imaging;
using System.Reflection;
using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LiteDB;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NAudio.Wave;
using NAudio.Wave.SampleProviders;
using Newtonsoft.Json.Linq;
using NonsPlayer.Core.AMLL.Parsers;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.DataBase;
using NonsPlayer.DataBase.Models;

namespace NonsPlayer.ViewModels;

public partial class TestViewModel : ObservableObject
{
    [RelayCommand]
    public async Task Test(string data)
    {
        var window = new CrashWindow(new NullReferenceException("test"));
        window.AppWindow.Show();
    }
}