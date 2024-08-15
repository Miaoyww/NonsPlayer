using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.Json;

namespace NonsPlayer.Services;

public class LocalService
{
    private const string _configKey = "local_dictionaries";
    private Dictionary<string, LocalFolderModel> _hashMap = new();
    public ObservableCollection<LocalFolderModel> Directories = new();

    public bool TryAddDirection(string name, string path)
    {
        if (_hashMap.ContainsKey(name)) return false;
        if (string.IsNullOrEmpty(name)) return false;
        var index = _hashMap.Count + 1;
        var model = new LocalFolderModel(name, path, index.ToString("D2"));
        _hashMap.Add(name, model);
        Directories.Add(model);
        Save();
        return true;
    }

    public bool TryDelDirection(string name)
    {
        if (!_hashMap.ContainsKey(name)) return false;
        Directories.Remove(_hashMap[name]);
        _hashMap.Remove(name);
        Save();
        return true;
    }

    public void LoadFromFile()
    {
        if (ConfigManager.Instance.TryGetConfig(_configKey,
                out var result))
        {
            var value = JObject.Parse(result);
            foreach (KeyValuePair<string, JToken?> keyValuePair in value)
            {
                _hashMap.Add(keyValuePair.Key, new LocalFolderModel(
                    ((JObject)keyValuePair.Value)["path"].ToString(),
                    ((JObject)keyValuePair.Value)["name"].ToString(),
                    (_hashMap.Count + 1).ToString("D2")
                ));
            }

            Directories.Clear();
            foreach (KeyValuePair<string, LocalFolderModel> item in _hashMap)
            {
                Directories.Add(item.Value);
            }
        }
        else
        {
            ConfigManager.Instance.SetConfig(_configKey, _hashMap);
        }
    }

    public void Save()
    {
        ConfigManager.Instance.SetConfig(_configKey, _hashMap);
        ConfigManager.Instance.Save();
    }
}