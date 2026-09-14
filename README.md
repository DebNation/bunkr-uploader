# 📦 bunkr-uploader

[![Crates.io](https://img.shields.io/crates/v/bunkr-uploader.svg)](https://crates.io/crates/bunkr-uploader)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

A fast CLI tool to upload files to bunkr.cr --- create & list albums,
upload files or entire directories (including sub-directories), and see
a progress bar while uploading.

------------------------------------------------------------------------

## Features

-   Create New Album
-   List Albums
-   Upload files
-   Upload directories and sub-directories recursively
-   Upload files individually or to an album
-   Upload progress bar
-   **yazi** file picker support (opens automatically when no paths are provided)

------------------------------------------------------------------------

## Requirements

You must have a **bunkr account** in order to upload files.

Get your account token from:

https://dash.bunkr.cr/dashboard

------------------------------------------------------------------------

## Installation

Install using Cargo:

``` bash
cargo install bunkr-uploader
```

Or download a **prebuilt binary** from the releases page:

https://github.com/debiplobi/bunkr-uploader/releases

------------------------------------------------------------------------

## Usage

``` bash
Usage: bunkr-uploader [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  path to files or directory

Options:
  -f             force upload without skipping(for special case)
  -h, --help     Print help
  -V, --version  Print version
```

------------------------------------------------------------------------

## Examples

Upload a single file:

``` bash
bunkr-uploader file.jpg
```

Upload a directory:

``` bash
bunkr-uploader ./images
```

Upload multiple files:

``` bash
bunkr-uploader file1.jpg file2.png video.mp4
```

Force upload:

``` bash
bunkr-uploader -f ./files
```

Use `yazi` file picker:

``` bash
bunkr-uploader
```

------------------------------------------------------------------------

## Notes

-   Files larger than **2GB** are rejected.
