﻿using Newtonsoft.Json.Linq;
using NonsPlayer.Components.Models;
using NonsPlayer.Core.Models;
using NonsPlayer.Core.Services;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.Json;

namespace NonsPlayer.Services;

public class LocalService
{
    private const string _dataKey = "local_dictionaries.json";
    public ObservableCollection<LocalFolderModel> Directories = new();
    public HashSet<LocalMusic> Songs = new();
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
        return true;
    }

    public void LoadFromFile()
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
}