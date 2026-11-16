PowerPico Client - Windows x86_64 portable edition

Extract all files to a folder, then run powerpico-client.exe.
The portable edition does not require an installer or administrator privileges.
Microsoft Edge WebView2 Runtime must already be installed on the computer.
If the runtime is missing, install it or use the NSIS setup edition, which can
download the runtime during setup.

This is an installation-free package. Settings, firmware downloads, recordings
and logs still use the current user's Windows AppData directories, as in the
installed edition. Moving this folder does not move your application data.

To update this edition, close the client and extract a newer portable ZIP into
the application folder. Keep exported recordings outside the application folder.
The in-app updater uses the NSIS installer and switches to the installed edition.
Verify downloads using the SHA256SUMS file supplied with the GitHub Release.

Source and releases: https://github.com/No-Chicken/PowerPicoClient_rs
License: GPL-3.0-only; see LICENSE.txt and THIRD_PARTY_NOTICES.md.

Windows x86_64 免安装版

解压全部文件后运行 powerpico-client.exe，无需安装或管理员权限。
电脑需要已有 Microsoft Edge WebView2 Runtime；缺少时请安装该运行时，
或使用能在安装过程中下载运行时的 NSIS 安装版。

本版本为免安装分发，设置、固件下载、采集记录和日志仍保存到当前用户的
Windows AppData 目录，与安装版共享；移动程序文件夹不会移动这些数据。
更新时请关闭客户端，再用新版本便携 ZIP 替换程序文件。导出记录请另存。
应用内更新使用 NSIS 安装器，会转为安装版。
下载后可使用 Release 中的 SHA256SUMS 校验文件完整性。
