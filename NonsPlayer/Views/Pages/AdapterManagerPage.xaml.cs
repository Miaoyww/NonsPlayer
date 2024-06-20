using Windows.Storage;
using Windows.Storage.AccessCache;
using Windows.Storage.Pickers;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NonsPlayer.ViewModels;

namespace NonsPlayer.Views.Pages;

public sealed partial class AdapterManagerPage : Page
{
    public AdapterManagerPage()
    {
        ViewModel = App.GetService<AdapterManagerViewModel>();
        InitializeComponent();
    }

    public async void PickFolder()
    {
        FolderPicker folderPicker = new()
        {
            SuggestedStartLocation = PickerLocationId.ComputerFolder
        };
        folderPicker.FileTypeFilter.Add("*");

        StorageFolder folder = await folderPicker.PickSingleFolderAsync();
        if (folder != null)
        {
            // Application now has read/write access to all contents in the picked folder
            // (including other sub-folder contents)
            StorageApplicationPermissions.FutureAccessList.AddOrReplace("PickedFolderToken", folder);
            // Use folder.Path to get the selected folder path
        }
        else
        {
            // Operation cancelled.
        }
    }

    private void LayoutToggleButton_Checked(object sender, RoutedEventArgs e)
    {
        
    }

    private void LayoutToggleButton_Unchecked(object sender, RoutedEventArgs e)
    {
    }

    public AdapterManagerViewModel ViewModel { get; }
}