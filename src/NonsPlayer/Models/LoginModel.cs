using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Contracts.Models.Nons;
using NonsPlayer.Helpers;

namespace NonsPlayer.Models;

public partial class LoginModel
{
    public IAccount Account;
    public IAdapter Adapter;
}