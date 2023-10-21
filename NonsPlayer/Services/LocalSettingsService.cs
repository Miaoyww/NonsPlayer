using Windows.Storage;
using Microsoft.Extensions.Options;
using NonsPlayer.Contracts.Services;
using NonsPlayer.Core.Contracts.Services;
using NonsPlayer.Core.Helpers;
using NonsPlayer.Helpers;
using NonsPlayer.Models;

namespace NonsPlayer.Services;

public class LocalSettingsService : ILocalSettingsService
{
    private const string _defaultApplicationDataFolder = "NonsPlayer/ApplicationData";
    private const string _defaultLocalSettingsFile = "LocalSettings.json";
    private readonly string _applicationDataFolder;

    private readonly IFileService _fileService;

    private readonly string _localApplicationData =
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);

    private readonly string _localsettingsFile;
    private readonly LocalSettingsOptions _options;

    private bool _isInitialized;

    private IDictionary<string, object> _settings;

    public LocalSettingsService(IFileService fileService, IOptions<LocalSettingsOptions> options)
    {
        _fileService = fileService;
        _options = options.Value;

        _applicationDataFolder = Path.Combine(_localApplicationData,
            _options.ApplicationDataFolder ?? _defaultApplicationDataFolder);
        _localsettingsFile = _options.LocalSettingsFile ?? _defaultLocalSettingsFile;

        _settings = new Dictionary<string, object>();
    }

    public async Task<T?> ReadSettingAsync<T>(string key)
    {
        if (RuntimeHelper.IsMSIX)
        {
            if (ApplicationData.Current.LocalSettings.Values.TryGetValue(key, out var obj))
                return await Json.ToObjectAsync<T>((string)obj);
        }
        else
        {
            await InitializeAsync();

            if (_settings != null && _settings.TryGetValue(key, out var obj))
                return await Json.ToObjectAsync<T>((string)obj);
        }

        return default;
    }

    public async Task SaveSettingAsync<T>(string key, T value)
    {
        if (RuntimeHelper.IsMSIX)
        {
            ApplicationData.Current.LocalSettings.Values[key] = await Json.StringifyAsync(value);
        }
        else
        {
            await InitializeAsync();

            _settings[key] = await Json.StringifyAsync(value);

            await Task.Run(() => _fileService.Save(_applicationDataFolder, _localsettingsFile, _settings));
        }
    }

    public LocalSettingsOptions GetOptions()
    {
        return _options;
    }

    private async Task InitializeAsync()
    {
        if (!_isInitialized)
        {
            _settings = await Task.Run(() =>
                            _fileService.Read<IDictionary<string, object>>(_applicationDataFolder,
                                _localsettingsFile)) ??
                        new Dictionary<string, object>();

            _isInitialized = true;
        }
    }
}