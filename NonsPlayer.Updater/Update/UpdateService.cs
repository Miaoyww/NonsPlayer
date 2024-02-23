using System.Buffers;
using System.Net;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using Microsoft.Extensions.Logging;
using NonsPlayer.Core.Services;
using NonsPlayer.Updater.Github;
using NonsPlayer.Updater.Metadata;
using NuGet.Versioning;
using SevenZipExtractor;

namespace NonsPlayer.Updater.Update;

public class UpdateService
{
    public enum UpdateState
    {
        Stop,

        Preparing,

        Pending,

        Downloading,

        Moving,

        Checking,

        Finish,

        Error,

        NotSupport
    }

    private readonly HttpClient _httpClient;
    private readonly ILogger<UpdateService> _logger;

    private readonly UpdateClient _updateClient;

    private ReleaseFile _downloadFile;

    private ReleaseVersion _releaseVersion;

    private string _appFolder;

    private string _unzipFolder;

    public long Progress_BytesToDownload { get; private set; }

    private long progress_BytesDownloaded;
    public long Progress_BytesDownloaded => progress_BytesDownloaded;


    private int progress_FileCountDownloaded;

    private List<LocalFile> localFiles;

    private List<LocalFile> targetFiles;


    public UpdateService(UpdateClient _inUpdateClient)
    {
        _httpClient = new HttpClient(new HttpClientHandler { AutomaticDecompression = DecompressionMethods.All })
            { DefaultRequestVersion = HttpVersion.Version20 };
        _updateClient = new UpdateClient();
    }

    public UpdateState State { get; private set; }
    
    public string ErrorMessage { get; set; }


    public async Task<ReleaseVersion?> CheckUpdateAsync(string version, Architecture architecture,
        bool enablePreviewRelease = false, bool disableIgnore = false)
    {
        NuGetVersion.TryParse(version, out var currentVersion);
        // _logger.LogInformation("Start to check update (Preview: {preview}, Arch: {arch})", AppConfig.EnablePreviewRelease, RuntimeInformation.OSArchitecture);
        var latestRelease = await _updateClient.GetLatestVersionAsync(enablePreviewRelease, architecture);
        // _logger.LogInformation("Current version: {0}, latest version: {1}, ignore version: {2}", AppConfig.AppVersion, release?.Version, ignoreVersion);
        NuGetVersion.TryParse(latestRelease.Version, out var latestVersion);
        if (latestVersion! > currentVersion!)
        {
            return latestRelease;
        }

        return null;
    }


    #region Prepare

    public async Task PrepareForUpdateAsync(ReleaseVersion release)
    {
        try
        {
            cancelSource?.Cancel();
            ErrorMessage = string.Empty;
            _releaseVersion = release;
            State = UpdateState.Preparing;
            _appFolder = AppContext.BaseDirectory;
            _unzipFolder = Path.Combine(_appFolder, "unzip");
            GetDownloadFile();
            progress_BytesDownloaded = 0;
            Progress_BytesToDownload = _downloadFile.Release.PortableSize;
            State = UpdateState.Pending;
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw(ex);
            // _logger.LogError(ex, "Prepare for update");
            State = UpdateState.Stop;
        }
    }

    private void GetDownloadFile()
    {
        var targetFilePath = Path.Combine(_appFolder,
            $"NonsPlayer_Portable_{_releaseVersion.Version}_{_releaseVersion.Architecture}.7z");
        var localFile = new LocalFile
        {
            Path = targetFilePath,
            To = _unzipFolder
        };
        var file = new ReleaseFile
        {
            Release = _releaseVersion,
            File = localFile
        };
        _downloadFile = file;
    }

    #endregion


    #region Update

    private CancellationTokenSource? cancelSource;


    public void Start()
    {
        new Thread(async () => await UpdateAsync()).Start();
    }


    public void Stop()
    {
        cancelSource?.Cancel();
        State = UpdateState.Stop;
        progress_BytesDownloaded = 0;
        Progress_BytesToDownload = 0;
    }


    public async Task UpdateAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            progress_BytesDownloaded = 0;
            Progress_BytesToDownload = _downloadFile.Release.PortableSize;
            cancelSource?.Cancel();
            cancelSource = new CancellationTokenSource();
            var source = cancelSource;
            State = UpdateState.Downloading;
            await DownloadFileAsync(_downloadFile, source.Token);
            if (source.IsCancellationRequested) throw new TaskCanceledException();

            var check = CheckDownloadFile();
            if (!check) throw new Exception("File verification failed");

            if (source.IsCancellationRequested) throw new TaskCanceledException();

            State = UpdateState.Moving;
            await Task.Delay(1000);
            await Task.Run(() =>
            {
                UnzipFile();
                MovingFolder();
            });
            State = UpdateState.Finish;
        }
        catch (TaskCanceledException)
        {
            State = UpdateState.Stop;
        }
        catch (Exception ex)
        {
            // _logger.LogError(ex, "Update failed");
            State = UpdateState.Error;
            ErrorMessage = ex.Message;
        }
    }


    private async Task DownloadFileAsync(ReleaseFile releaseFile, CancellationToken cancellationToken = default)
    {
        var readLength = 0;
        for (var i = 0; i < 3; i++)
            try
            {
                readLength = 0;
                var file = _downloadFile.File.Path;
                if (!File.Exists(file))
                {
                    var request = new HttpRequestMessage(HttpMethod.Get, releaseFile.Release.Portable)
                        { Version = HttpVersion.Version11 };
                    var response = await _httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead,
                        cancellationToken);
                    response.EnsureSuccessStatusCode();
                    using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
                    var ms = new MemoryStream();
                    var buffer = new byte[1 << 16];
                    int length;
                    while ((length = await stream.ReadAsync(buffer, cancellationToken)) != 0)
                    {
                        ms.Write(buffer, 0, length);
                        readLength += length;
                        Interlocked.Add(ref progress_BytesDownloaded, length);
                    }

                    await File.WriteAllBytesAsync(file, ms.ToArray(), cancellationToken);
                    Interlocked.Increment(ref progress_FileCountDownloaded);
                }

                using var fs = File.Open(file, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                var hash = Convert.ToHexString(await SHA256.HashDataAsync(fs, cancellationToken));
                if (!string.Equals(hash, releaseFile.Release.PortableHash, StringComparison.OrdinalIgnoreCase))
                {
                    _logger.LogWarning("Checksum failed, path: {path}, actual hash: {hash}, true hash: {truehash}",
                        releaseFile.File.Path, hash, releaseFile.Release.PortableHash);
                    throw new Exception($"Checksum failed: {releaseFile.File.Path}");
                }

                break;
            }
            catch (TaskCanceledException)
            {
                Interlocked.Add(ref progress_BytesDownloaded, -readLength);
                break;
            }
            catch (HttpRequestException ex)
            {
                _logger.LogWarning("Download failed: {error}\r\n{url}", ex.Message, releaseFile.Release.Portable);
                Interlocked.Add(ref progress_BytesDownloaded, -readLength);
                if (i == 2) throw;
            }
    }


    private bool CheckDownloadFile()
    {
        var file = _downloadFile.File.Path;
        if (!File.Exists(file)) return false;

        return true;
    }

    private void UnzipFile()
    {
        Directory.CreateDirectory(_appFolder);
        using (var archiveFile = new ArchiveFile(_downloadFile.File.Path))
        {
            archiveFile.Extract(_downloadFile.File.To);
        }
    }

    private void MovingFolder()
    {
        try
        {
            var tagertFolder = new DirectoryInfo(AppContext.BaseDirectory).Parent?.FullName;
            var appVersionFolder = $"app-{_downloadFile.Release.Version}";
            File.Copy(Path.Combine(_unzipFolder, "NonsPlayer", "version.ini"),
                Path.Combine(tagertFolder, "version.ini"),
                overwrite: true);
            var launcherFile = Path.Combine(tagertFolder, "NonsPlayer.exe");
            if (!File.Exists(launcherFile))
            { 
                File.Copy(Path.Combine(_unzipFolder, "NonsPlayer", "NonsPlayer.exe"),
                    launcherFile,
                    overwrite: true);
            }
            
            Directory.Move(Path.Combine(_unzipFolder, "NonsPlayer", appVersionFolder),
                Path.Combine(tagertFolder, appVersionFolder));
        }
        catch (Exception ex)
        {
            ExceptionService.Instance.Throw("更新失败~建议手动下载并覆盖");
        }
    }

    #endregion
}