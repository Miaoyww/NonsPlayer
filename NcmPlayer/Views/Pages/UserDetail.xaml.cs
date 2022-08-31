using NcmPlayer.Resources;
using System.Windows.Controls;

namespace NcmPlayer.Views.Pages
{
    /// <summary>
    /// UserDetail.xaml 的交互逻辑
    /// </summary>
    public partial class UserDetail : Page
    {
        public UserDetail()
        {
            InitializeComponent();
            DataContext = ResEntry.res;
        }
    }
}