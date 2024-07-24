using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;
using TagLib.Id3v2;

namespace NonsPlayer.Views.Pages;

public sealed partial class PersonalCenterPage : Page
{
    public PersonalCenterPage()
    {
        ViewModel = App.GetService<PersonalCenterViewModel>();
        InitializeComponent();
        RefreshInfo();
    }

    public PersonalCenterViewModel ViewModel { get; }

    private void RefreshInfo()
    {
        SelectorBar.Items.Clear();
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Account);
        foreach (var adapter in adapters)
        {
            var metaData = adapter.GetMetadata();
            SelectorBar.Items.Add(
                new SelectorBarItem
                {
                    Text = metaData.DisplayPlatform,
                    Tag = adapter,
                    Style = App.Current.Resources["CustomSelectorBarItem"] as Style,
                });
        }

        SelectorBar.Items.Add(new()
        {
            Text = "SettingsText".GetLocalized(),
            Tag = "setting",
            Style = App.Current.Resources["CustomSelectorBarItem"] as Style,
        });
    }

    private async void SelectorBar_OnSelectionChanged(SelectorBar sender, SelectorBarSelectionChangedEventArgs args)
    {
        var selectedItem = sender.SelectedItem as SelectorBarItem;
        var currentSelectedIndex = sender.Items.IndexOf(selectedItem);
        if (selectedItem.Tag is string)
        {
            return;
        }

        var adapter = selectedItem.Tag as IAdapter;
        if (adapter.Account.GetAccount().IsLoggedIn)
        {
            Navigate(typeof(PersonalLibaryPage), adapter);
        }
        else
        {
            Navigate(typeof(LoginPage), adapter);
        }
    }

    private void Navigate(Type pageType, object parameter)
    {
        var navigated = ContentFrame.Navigate(pageType, parameter);
        var vmBeforeNavigation = ContentFrame.GetPageViewModel();
        if (navigated)
        {
            if (vmBeforeNavigation is INavigationAware navigationAware) navigationAware.OnNavigatedTo(parameter);
        }
    }
}