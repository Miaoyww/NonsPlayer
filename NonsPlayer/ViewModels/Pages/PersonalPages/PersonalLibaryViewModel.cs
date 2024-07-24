using CommunityToolkit.Mvvm.ComponentModel;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;

namespace NonsPlayer.ViewModels;

public partial class PersonalLibaryViewModel : ObservableRecipient, INavigationAware
{
    public PersonalLibaryViewModel()
    {
    }

    public void OnNavigatedTo(object parameter)
    {
        var adapter = parameter as IAdapter;
    }

    public void OnNavigatedFrom()
    {
    }
}
