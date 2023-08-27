using System.Diagnostics;

namespace NonsPlayer.Updater;

public class UpdaterClient
{
    private void RunShell()
    {
        // ref: https://github.com/Scighost/WinUI3Keng/tree/main/%E8%87%AA%E5%8A%A8%E6%9E%84%E5%BB%BA%E4%B8%8E%E8%87%AA%E5%8A%A8%E6%9B%B4%E6%96%B0
        const string script = @"
        # 出现错误时停止执行后续命令，若不加这一句，即使出现错误也无法被 catch
        # 代码省略了最外部的 try-catch 部分
        $ErrorActionPreference = 'Stop'
        # 检查软件是否仍在运行
        try {
            # 没找到进程时会抛错，需要 catch
            $null = Get-Process -Name ""NonsPlayer""
            Write-Host ""Nonsplayer.exe 正在运行，等待进程退出"" -ForegroundColor Yellow
            Wait-Process -Name ""NonsPlayer""
            # 停 1s 等待资源释放
            Start-Sleep -Seconds 1
        } catch { }
        .\Install.ps1
        # 清理安装包
        Remove-Item -Path ""./temp"" -Force -Recurse";
        Process.Start("PowerShell", script);
    }

    public static async Task Run(Version version)
    {
        var currentVersion = $"v{version.Major}.{version.Minor}.{version.Build}";
    }
}