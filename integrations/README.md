# Integrations

This directory contains integrations between IronTBL and third party software.


## Shell completion

IronTBL has built in support for generating shell completions, using the `--generate-shell-completions` flag. To generate completions for zsh, for example, run this:

```
$ ./irontbl --generate-shell-completions zsh | sudo tee /usr/local/share/zsh/site-functions/_irontbl
$ compinit
```


## ImHex

[ImHex](https://github.com/WerWolv/ImHex) is a very capable open source hex editor. Included in the `imhex` directory is a hexpat file for parsing and highlighting .tbl files.


## Yazi

[Yazi](https://yazi-rs.github.io/) is an open source terminal file manager. Included here is a plugin that integrates IronTBL to the preview panel of Yazi, so that the content of .tbl files can be seen when they are selected. Move the `irontbl.yazi` directory to `~/.config/yazi/plugins`. Then add this to `~.config/yazi/yazi.toml`:

```
[plugin]
prepend_previewers = [
    { name = "*.tbl", run = "irontbl"},
    { name = "*.str", run = "irontbl"},
]
```

The .str extension is from the WarCraft II modding days before the Battle.net edition; if you don't anticipate that you will interact with WarCraft II mods, you can choose to only keep the prepend previewer for .tbl files.
