using Microsoft.UI.Xaml.Input;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.ViewModels;
using NonsPlayer.Core.Models;
using NonsPlayer.Helpers;
using UserControl = Microsoft.UI.Xaml.Controls.UserControl;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace NonsPlayer.Components.Views
{
    public sealed partial class UserPlaylistCard : UserControl
    {
        public UserPlaylistCardViewModel ViewModel
        {
            get;
        }

        public Playlist PlaylistItem
        {
            set => ViewModel.Init(value);
        }
        public UserPlaylistCard()
        {
            ViewModel = App.GetService<UserPlaylistCardViewModel>();
            InitializeComponent();
        }
        private void CardShow(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardShow(sender, e);
        private void CardHide(object sender, PointerRoutedEventArgs e) => AnimationsHelper.CardHide(sender, e);

    }
}