using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Windows.Forms;
using Windows.Foundation;
using Windows.Foundation.Collections;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.ViewModels;
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

        public JObject PlaylistItem
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