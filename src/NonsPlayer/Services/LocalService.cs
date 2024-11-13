using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Core.Utils;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.Json;
using Windows.Storage;
using Exception = System.Exception;
using FileAttributes = Windows.Storage.FileAttributes;

namespace NonsPlayer.Services;

public class LocalService
{
    #region 事件注册

    public delegate void LocalFolderModelEventHandler(string param);

    public event LocalFolderModelEventHandler? LocalFolderChanged;
    public event LocalFolderModelEventHandler? LocalLoadFailed;

    #endregion

    private const string _dataKey = "local_dictionaries.json";
    public ObservableCollection<LocalFolderModel> Directories = new();
    public List<LocalMusic> Songs = new();
    public List<LocalArtist> Artists = new();
    public List<LocalAlbum> Albums = new();

    private FileService FileService = App.GetService<FileService>();

    public bool TryAddDirection(string path)
    {
        if (string.IsNullOrEmpty(path)) return false;
        if (HasDirectory(path)) return false;
        var index = Directories.Count;
        var model = new LocalFolderModel(path, index.ToString("D2"));
        Directories.Add(model);
        Save();
        return true;
    }

    public void AddSong(LocalMusic song)
    {
        foreach (LocalMusic songItem in Songs)
        {
            if (songItem.FilePath.Equals(song.FilePath)) return;
        }

        LocalFolderChanged?.Invoke(string.Empty);
        Songs.Add(song);
    }

    public void AddSongs(IEnumerable<LocalMusic> songs)
    {
        foreach (LocalMusic inputSongItem in songs)
        {
            AddSong(inputSongItem);
        }

        LocalFolderChanged?.Invoke(string.Empty);
    }

    public bool HasDirectory(string path)
    {
        return Directories.Any(localFolderModel => localFolderModel.Path.Equals(Path.GetFullPath(path)));
    }

    public bool TryGetModel(string path, out LocalFolderModel result)
    {
        foreach (LocalFolderModel localFolderModel in Directories)
        {
            if (localFolderModel.Path.Equals(Path.GetFullPath(path)))
            {
                result = localFolderModel;
                return true;
            }
        }

        result = default;
        return false;
    }

    public bool TryDelDirection(string path)
    {
        if (!HasDirectory(path)) return false;
        if (!TryGetModel(path, out var result)) return false;
        Directories.Remove(result);
        Save();
        LocalFolderChanged?.Invoke(string.Empty);
        return true;
    }

    public void LoadFromFile()
    {
        try
        {
            var data = FileService.ReadData(_dataKey);
            if (!string.IsNullOrEmpty(data))
            {
                var value = JArray.Parse(data);
                Directories.Clear();
                var index = 0;
                foreach (var item in value)
                {
                    index++;
                    Directories.Add(new LocalFolderModel(
                        (item)["path"].ToString(),
                        index.ToString("D2")
                    ));
                }

                LocalFolderChanged?.Invoke(string.Empty);
            }
        }catch(Exception e)
        {
            LocalLoadFailed?.Invoke(e.Message);
        }

    }

    public void Save()
    {
        var options = new JsonSerializerOptions
        {
            Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
        };

        FileService.SaveData(_dataKey, JsonSerializer.Serialize(Directories, options));
    }
    
    public async Task<List<LocalMusic>> ScanMusic(string folderPath)
    {
        StorageFolder folder = await StorageFolder.GetFolderFromPathAsync(folderPath);
        var items = await folder.GetItemsAsync();
        List<LocalMusic> musics = new();
        foreach (var file in items)
        {
            if (file.Attributes.HasFlag(FileAttributes.Directory))
            {
                musics.AddRange(await ScanMusic(await (file as StorageFolder).GetItemsAsync()));
            }
            else if (file.Attributes.HasFlag(FileAttributes.Normal))
            {
                if (LocalUtils.IsMusic(file.Path))
                {
                    musics.Add(new LocalMusic(file.Path));
                }
            }
        }

        return musics;
    }

    public async Task<List<LocalMusic>> ScanMusic(IReadOnlyList<IStorageItem> items)
    {
        List<LocalMusic> musics = new();
        foreach (var file in items)
        {
            if (file.Attributes.HasFlag(Windows.Storage.FileAttributes.Directory))
            {
                musics.AddRange(await ScanMusic(await (file as StorageFolder).GetItemsAsync()));
            }
            else if (file.Attributes.HasFlag(FileAttributes.Normal))
            {
                if (LocalUtils.IsMusic(file.Path))
                {
                    musics.Add(new LocalMusic(file.Path));
                }
            }
        }

        return musics;
    }
}