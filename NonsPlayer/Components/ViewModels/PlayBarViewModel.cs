using System.ComponentModel;
using System.Runtime.CompilerServices;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Heplers;
using NonsPlayer.Data;

namespace NonsPlayer.Components.ViewModels
{
    public class PlayerBarViewModel : INotifyPropertyChanged
    {

        public MetaData MetaData => MetaData.Instance;
        public UserPlaylistHelper UserPlaylistHelper => UserPlaylistHelper.Instance;

        public PlayerBarViewModel()
        {
            MetaData.Instance.Volume = double.Parse(RegHelper.Instance.Get(RegHelper.Regs.Volume, 0.0).ToString());
        }
        public void Init()
        {

        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}