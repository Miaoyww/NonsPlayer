using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Runtime.InteropServices;
using System.Text.Json;
using NonsPlayer.Updater.Github;

namespace NonsPlayer.Updater.Service;

public class UpdaterClient
{
    private const string API_PREFIX_GITHUB = "https://raw.githubusercontent.com/Miaoyww/NonsPlayer/metadata";

    private string API_PREFIX = API_PREFIX_GITHUB;

    private const string API_VERSION = "v1";

    private readonly HttpClient _httpClient;

    public UpdaterClient(HttpClient? httpClient = null)
    {
        _httpClient = httpClient ??
                      new HttpClient(new HttpClientHandler { AutomaticDecompression = DecompressionMethods.All })
                          { DefaultRequestVersion = HttpVersion.Version20 };
    }


    private string GetUrl(string suffix)
    {
        return $"{API_PREFIX}/{API_VERSION}/{suffix}";
    }


    public async Task<GithubVersion> GetVersionAsync(bool isPrerelease, Architecture architecture)
    {
#if DEV
        isPrerelease = true;
#endif
        var name = (isPrerelease, architecture) switch
        {
            (false, Architecture.X64) => "version_stable_x64.json",
            (true, Architecture.X64) => "version_preview_x64.json",
            (false, Architecture.Arm64) => "version_stable_arm64.json",
            (true, Architecture.Arm64) => "version_preview_arm64.json",
            _ => throw new PlatformNotSupportedException($"{architecture} is not supported."),
        };
        var url = GetUrl(name);
        return await ParseGithubVersion(url);
    }

    private async Task<GithubVersion> ParseGithubVersion(string url)
    {
        var res = await _httpClient.GetFromJsonAsync<GithubVersion>(url);
        if (res == null)
        {
            throw new NullReferenceException($"检查更新失败, 获取的GithubVersion为空.URL= {url}");
        }

        return res;
    }
}