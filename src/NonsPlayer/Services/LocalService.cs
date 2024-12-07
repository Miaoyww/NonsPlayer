using Microsoft.Extensions.Logging;
using Microsoft.VisualBasic;
using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Contracts.Models.Music;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using NonsPlayer.Core.Utils;
using NonsPlayer.DataBase;
using NonsPlayer.DataBase.Models;
using System.Collections.ObjectModel;
using System.Diagnostics;
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

    public ObservableCollection<LocalFolderModel> Directories = new();
    public List<LocalMusic> Songs = new();
    public List<LocalArtist> Artists = new();
    public List<LocalAlbum> Albums = new();
    private ILogger _logger = App.GetLogger<LocalService>();
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
        var deleted = LocalDbManager.GetFolders().DeleteMany(x => x.Path == path);
        Save();
        LocalFolderChanged?.Invoke(string.Empty);
        return true;
    }

    public void LoadFromFile()
    {
        try
        {
            var data = LocalDbManager.GetFolders().FindAll();
            if (data != null)
            {
                Directories.Clear();
                var index = 0;
                foreach (var item in data)
                {
                    index++;
                    Directories.Add(new LocalFolderModel(
                        item.Path,
                        index.ToString("D2")
                    ));

                    _logger.LogInformation($"Loaded folder: {item.Path}");
                }

                LocalFolderChanged?.Invoke(string.Empty);
            }
        }
        catch (Exception e)
        {
            LocalLoadFailed?.Invoke(e.Message);
        }
    }

    public void Save()
    {
        var db = LocalDbManager.Instance.GetDatabase().GetCollection<DbFolderModel>("folders");
        db.EnsureIndex(x => x.Path, true); // 'true' 表示唯一索引

        foreach (LocalFolderModel localFolderModel in Directories)
        {
            try
            {
                LocalDbManager.UpdateFolder(localFolderModel.ConvertToDbFolderModel());
            }
            catch
            {
                //ignore;
            }

        }
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