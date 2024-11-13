using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Animation;
using NonsPlayer.Contracts.ViewModels;
using NonsPlayer.Core.Contracts.Adapters;
using NonsPlayer.Core.Services;
using NonsPlayer.Helpers;
using NonsPlayer.ViewModels;

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
        CustomSelectorBar.Items.Clear();
        var adapters = AdapterService.Instance.GetAdaptersByType(ISubAdapterEnum.Account);
        foreach (var adapter in adapters)
        {
            var metaData = adapter.GetMetadata();
            CustomSelectorBar.Items.Add(
                new SelectorBarItem
                {
                    Text = metaData.DisplayPlatform,
                    Tag = adapter,
                    Padding = new Thickness(0),
                    Margin = new Thickness(0,0,20,0),
                    Style = App.Current.Resources["CustomSelectorBarItem"] as Style,
                });
        }

        CustomSelectorBar.Items.Add(new()
        {
            Text = "SettingsText".GetLocalized(),
            Tag = "setting",
            Padding = new Thickness(0),
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
    
    public void PersonalCenterPage_OnLoaded(object sender, RoutedEventArgs e)
    {
        var defaultAdapter = AdapterService.Instance.GetAdapter(ConfigManager.Instance.Settings.DefaultAdapter);
        foreach (SelectorBarItem item in CustomSelectorBar.Items)
        {
            if (item.Tag is IAdapter adapter)
            {
                if (adapter == defaultAdapter)
                {
                    CustomSelectorBar.SelectedItem = item;
                    break;
                }
            }
        }
        //TODO: 本地账号设置
        // if (!Account.Instance.IsLoggedIn) NavigationService.NavigateTo(typeof(LoginViewModel).FullName!);
    }
}