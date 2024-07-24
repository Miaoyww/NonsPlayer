using System.Collections.ObjectModel;
using System.Drawing.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Newtonsoft.Json.Linq;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Core.Nons;
using NonsPlayer.Helpers;
using NonsPlayer.Models;
using QRCoder;

namespace NonsPlayer.ViewModels;

public partial class LoginViewModel : ObservableObject, INavigationAware
{
    public ObservableCollection<LoginModel> LoginModels;
    [ObservableProperty] private string platForm;
    [ObservableProperty] private ImageBrush avatar;

    public IAdapter Adapter;

    public async void OnNavigatedTo(object parameter)
    {
        Adapter = parameter as IAdapter;
        if (Adapter != null)
        {
            PlatForm = Adapter.GetMetadata().DisplayPlatform;
            Avatar = await CacheHelper.GetImageBrushAsync(
                Adapter.Account.GetAccount().CacheAvatarId, 
                await Adapter.Account.GetAccount().GetAvatarUrl());
        }
    }

    public void OnNavigatedFrom()
    {
    }
}