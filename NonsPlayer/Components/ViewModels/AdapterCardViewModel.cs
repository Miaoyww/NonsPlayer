using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.Components.ViewModels;

[INotifyPropertyChanged]
public partial class AdapterCardViewModel
{
    public AdapterMetadata Metadata { get; set; }
}