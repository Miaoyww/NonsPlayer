using ATL;
using NonsPlayer.Core.Resources;
using System.Text.Json;
using System.Text;

namespace NonsPlayer.Core.Contracts.Managers;

public interface IConfigManager
{
    void Load();
    void Save();
}