using System.Buffers;
using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;
using NonsPlayer.Updater.Github;
using NuGet.Versioning;

namespace NonsPlayer.Updater.Update;

public class UpdateService
{
    private readonly ILogger<UpdateService> _logger;

    private readonly HttpClient _httpClient;

    private readonly UpdateClient _updateClient;

    private string _updateFolder;


    private GithubVersion _releaseVersion;


    private List<ReleaseFile> _localFiles;


    private ReleaseFile _downloadFile;

    public UpdateService(ILogger<UpdateService> logger, HttpClient httpClient, UpdateClient _inUpdateClient)
    {
        _logger = logger;
        _httpClient = httpClient;
        _updateClient = _inUpdateClient;
    }


    public async Task<GithubVersion?> CheckUpdateAsync(Version currentVersion, Architecture architecture,
        bool enablePreviewRelease = false, bool disableIgnore = false)
    {
        // _logger.LogInformation("Start to check update (Preview: {preview}, Arch: {arch})", AppConfig.EnablePreviewRelease, RuntimeInformation.OSArchitecture);
        var latestRelease = await _updateClient.GetLatestVersionAsync(enablePreviewRelease, architecture);
        // _logger.LogInformation("Current version: {0}, latest version: {1}, ignore version: {2}", AppConfig.AppVersion, release?.Version, ignoreVersion);
        Version? latestVersion;
        Version.TryParse(latestRelease.Version, out latestVersion);
        if (currentVersion.CompareTo(latestVersion) < 0)
        {
            return latestRelease;
        }

        return null;
    }


    public UpdateState State { get; private set; }

    public int Progress_FileCountToDownload { get; private set; }

    private int progress_FileCountDownloaded;
    public int Progress_FileCountDownloaded => progress_FileCountDownloaded;

    public long Progress_BytesToDownload { get; private set; }


    private long progress_BytesDownloaded;
    public long Progress_BytesDownloaded => progress_BytesDownloaded;


    public string ErrorMessage { get; set; }


    #region Prepare

    public async Task PrepareForUpdateAsync(GithubVersion release)
    {
        try
        {
            cancelSource?.Cancel();
            ErrorMessage = string.Empty;
            _releaseVersion = release;
            State = UpdateState.Preparing;
            var baseFolder = new DirectoryInfo(AppContext.BaseDirectory).Parent?.FullName;
            if (baseFolder == null)
            {
                // 无法自动更新
                // ErrorMessage = Lang.UpdateService_CannotUpdateAutomatically;
                State = UpdateState.NotSupport;
                return;
            }

            var exe = Path.Join(baseFolder, "NonsPlayer.exe");
            if (!File.Exists(exe))
            {
                // 无法自动更新
                // ErrorMessage = Lang.UpdateService_CannotUpdateAutomatically;
                State = UpdateState.NotSupport;
                return;
            }

            _updateFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "NonsPlayer\\data");
            Directory.CreateDirectory(_updateFolder);
            await Task.Run(() =>
            {
                GetLocalFilesHash();
                GetDownloadFile();
            });
            progress_BytesDownloaded = 0;
            progress_FileCountDownloaded = 0;
            Progress_BytesToDownload = _downloadFile.Size;
            State = UpdateState.Pending;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Prepare for update");
            State = UpdateState.Stop;
        }
    }


    public void GetLocalFilesHash()
    {
        if (_localFiles is null)
        {
            var files = Directory.GetFiles(AppContext.BaseDirectory, "*", SearchOption.AllDirectories);
            var releaseFiles = new List<ReleaseFile>(files.Length);
            foreach (var file in files)
            {
                using var fs = File.Open(file, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
                var len = (int)fs.Length;
                var bytes = ArrayPool<byte>.Shared.Rent(len);
                fs.Read(bytes, 0, len);
                var span = bytes.AsSpan(0, len);
                var sha256 = SHA256.HashData(span);
                ArrayPool<byte>.Shared.Return(bytes);
                releaseFiles.Add(new ReleaseFile
                {
                    Path = file,
                    Size = len,
                    Hash = Convert.ToHexString(sha256)
                });
            }

            _localFiles = releaseFiles;
        }
    }

    private void GetDownloadFile()
    {
        var targetFilePath = Path.Combine(_updateFolder, _releaseVersion.PortableHash);

        var file = new ReleaseFile
        {
            Hash = _releaseVersion.PortableHash,
            Size = _releaseVersion.PortableSize,
            Path = targetFilePath
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
        progress_FileCountDownloaded = 0;
        Progress_BytesToDownload = 0;
        Progress_FileCountToDownload = 0;
    }


    public async Task UpdateAsync()
    {
        try
        {
            cancelSource?.Cancel();
            cancelSource = new();
            var source = cancelSource;
            State = UpdateState.Downloading;
            await DownloadFilesAsync(source.Token);
            if (source.IsCancellationRequested)
            {
                throw new TaskCanceledException();
            }

            var check = CheckDownloadFile();
            if (!check)
            {
                throw new Exception("File verification failed");
            }

            if (source.IsCancellationRequested)
            {
                throw new TaskCanceledException();
            }

            State = UpdateState.Moving;
            await Task.Delay(1000);
            MovingFile();
            State = UpdateState.Finish;
        }
        catch (TaskCanceledException)
        {
            State = UpdateState.Stop;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Update failed");
            State = UpdateState.Error;
            ErrorMessage = ex.Message;
        }
    }


    private async Task DownloadFilesAsync(CancellationToken cancellationToken = default)
    {
        progress_BytesDownloaded = 0;
        progress_FileCountDownloaded = 0;
        Progress_BytesToDownload = _downloadFile.Size;
        await DownloadFileAsync(_downloadFile, cancellationToken);
    }


    private async Task DownloadFileAsync(ReleaseFile releaseFile, CancellationToken cancellationToken = default)
    {
        var readLength = 0;
        for (int i = 0; i < 3; i++)
        {
            try
            {
                readLength = 0;
                var file = Path.Combine(_updateFolder, releaseFile.Hash);
                if (!File.Exists(file))
                {
                    var request = new HttpRequestMessage(HttpMethod.Get, releaseFile.Url)
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
                string hash = Convert.ToHexString(await SHA256.HashDataAsync(fs, cancellationToken));
                if (!string.Equals(hash, releaseFile.Hash, StringComparison.OrdinalIgnoreCase))
                {
                    _logger.LogWarning("Checksum failed, path: {path}, actual hash: {hash}, true hash: {truehash}",
                        releaseFile.Path, hash, releaseFile.Hash);
                    throw new Exception($"Checksum failed: {releaseFile.Path}");
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
                _logger.LogWarning("Download failed: {error}\r\n{url}", ex.Message, releaseFile.Url);
                Interlocked.Add(ref progress_BytesDownloaded, -readLength);
                if (i == 2)
                {
                    throw;
                }
            }
        }
    }


    private bool CheckDownloadFile()
    {
        var file = Path.Combine(_updateFolder, _downloadFile.Hash);
        if (!File.Exists(file))
        {
            return false;
        }

        return true;
    }


    private void MovingFile()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(_downloadFile.To)!);
        if (_downloadFile.IsMoving)
        {
            File.Move(_downloadFile.From, _downloadFile.To, true);
        }
        else
        {
            File.Copy(_downloadFile.From, _downloadFile.To, true);
        }
    }

    #endregion


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

        NotSupport,
    }
}