# 教学立方课件下载器——Rust Tokio版

<p align="center">
  <img alt="GitHub" src="https://img.shields.io/github/license/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/TwinklerG/PedagogySquare-Downloader-rs">	
  <img alt="GitHub release (latest by date)" src="https://img.shields.io/github/v/release/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub code size in bytes" src="https://img.shields.io/github/languages/code-size/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub top language" src="https://img.shields.io/github/languages/top/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub stars" src="https://img.shields.io/github/stars/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub All Releases" src="https://img.shields.io/github/downloads/TwinklerG/PedagogySquare-Downloader-rs/total">
  <img alt="GitHub issues" src="https://img.shields.io/github/issues-raw/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="GitHub closed issues" src="https://img.shields.io/github/issues-closed-raw/TwinklerG/PedagogySquare-Downloader-rs">
  <img alt="PRs welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen">
</p>
在线教学平台——[教学立方](https://teaching.applysquare.com)的课件批量并行下载脚本，基于**Rust** + **Reqwest** + **Tokio**

本项目重写自EricZhu学长的项目[PedagogySquare_Downloader](https://github.com/EricZhu-42/PedagogySquare_Downloader)

主要优势：充分利用Rust无畏并发的特点和Tokio的强大性能，拥有更快的下载速度和极强的并发量。

## 使用说明

在本项目中的Release部分下载对应平台的压缩包，解压缩后可以看到有两个文件

- 可执行文件
- `config.json`配置文件

**首先需要编辑配置文件**

```json
{
  "username": "你的账号",
  "password": "你的密码",
  "ext_expel_list": ["pdf", "ppt"],
  "cid_expel_list": ["114514"],
  "cid_include_list": ["1919810", "30860"]
}
```

- username, password分别是你的用户名和密码，需要双引号确保json合法
- ext_expel_list是排除的文件后缀名，不会下载拥有这些后缀名的文件
- cid_expel_list是排除的课程ID列表，如果指定则不会下载指定课程号的文件
- cid_include_list是包含的课程ID列表，如果为空则会下载**所有**课程，如果指定则只会下载指定课程号的文件

**然后便可启动下载器**

windows下双击即可运行，linux/macos建议采用命令行