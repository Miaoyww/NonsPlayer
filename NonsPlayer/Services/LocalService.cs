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
    private Dictionary<string, LocalFolderModel> _dictHashMap = new();
    public ObservableCollection<LocalFolderModel> Directories = new();
    public HashSet<LocalMusic> Songs = new();

    public bool TryAddDirection(string name, string path)
    {
        if (_dictHashMap.ContainsKey(name)) return false;
        if (string.IsNullOrEmpty(name)) return false;
        var index = _dictHashMap.Count + 1;
        var model = new LocalFolderModel(name, path, index.ToString("D2"));
        _dictHashMap.Add(name, model);
        Directories.Add(model);
        Save();
        return true;
    }

    public bool TryAddSong(LocalMusic song)
    {
        foreach (LocalMusic songItem in Songs)
        {
            if (songItem.FilePath.Equals(song.FilePath)) return false;
        }
        
        return Songs.Add(song);
    }  

    public void AddSongs(IEnumerable<LocalMusic> songs)
    {
        foreach (LocalMusic inputSongItem in songs)
        {
            TryAddSong(inputSongItem);
        }
    }

    public bool TryDelDirection(string name)
    {
        if (!_dictHashMap.ContainsKey(name)) return false;
        Directories.Remove(_dictHashMap[name]);
        _dictHashMap.Remove(name);
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
                _dictHashMap.Add(keyValuePair.Key, new LocalFolderModel(
                    ((JObject)keyValuePair.Value)["name"].ToString(),
                    ((JObject)keyValuePair.Value)["path"].ToString(),
                    (_dictHashMap.Count + 1).ToString("D2")
                ));
            }

            Directories.Clear();
            foreach (KeyValuePair<string, LocalFolderModel> item in _dictHashMap)
            {
                Directories.Add(item.Value);
            }
        }
        else
        {
            ConfigManager.Instance.SetConfig(_configKey, _dictHashMap);
        }
    }

    public void Save()
    {
        ConfigManager.Instance.SetConfig(_configKey, _dictHashMap);
        ConfigManager.Instance.Save();
    }
}