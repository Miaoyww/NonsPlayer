using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using NonsPlayer.Updater.Github;

namespace NonsPlayer.Updater.Update;

public class UpdateClient
{
    public bool IsUpdated { get; set; }

    private const string API_PREFIX_GITHUB = "https://raw.githubusercontent.com/Miaoyww/NonsPlayer/metadata";

    private string API_PREFIX = API_PREFIX_GITHUB;

    private const string API_VERSION = "v1";

    private readonly HttpClient _httpClient;

    public UpdateClient(HttpClient? httpClient = null)
    {
        _httpClient = httpClient ??
                      new HttpClient(new HttpClientHandler { AutomaticDecompression = DecompressionMethods.All })
                          { DefaultRequestVersion = HttpVersion.Version20 };
    }


    private string GetUrl(string suffix)
    {
        return $"{API_PREFIX}/{API_VERSION}/{suffix}";
    }


    public async Task<ReleaseVersion> GetLatestVersionAsync(bool isPrerelease, Architecture architecture)
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
        return await ParseResponse<ReleaseVersion>(url);
    }

    public async Task<GithubRelease?> GetGithubReleaseAsync(string tag)
    {
        string url = $"https://api.github.com/repos/Miaoyww/NonsPlayer/releases/tags/{tag}";
        return await ParseResponse<GithubRelease>(url);
    }

    private async Task<T> ParseResponse<T>(string url) where T : class
    {
        var res =  await _httpClient.GetFromJsonAsync(url, typeof(T)) as T;
        if (res == null)
        {
            throw new NullReferenceException($"尝试ParseResponse失败,res为空.URL= {url}");
        }

        return res;
    }

    public async Task<string> RenderGithubMarkdownAsync(string markdown, CancellationToken cancellationToken = default)
    {
        const string url = "https://api.github.com/markdown";
        var request = new GithubMarkdown
        {
            Text = markdown,
            Mode = "gfm",
            Context = "Miaoyww/NonsPlayer",
        };
        var options = new JsonSerializerOptions { WriteIndented = true };
        var content =
            new StringContent(
                JsonSerializer.Serialize(request, typeof(GithubMarkdown), options),
                new MediaTypeHeaderValue("application/json"));
        var response = await _httpClient.PostAsync(url, content, cancellationToken);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }
}