# 📫 Himalaya.vim

Vim plugin for [Himalaya](https://github.com/soywod/himalaya) CLI email client.

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

## Table of contents

* [Motivation](#motivation)
* [Installation](#installation)
* [Usage](#usage)
  * [List messages view](#list-messages-view)
  * [List mailboxes](#list-mailboxes)
  * [Read message view](#read-message-view)
  * [Write message view](#write-message-view)
* [License](https://github.com/soywod/himalaya.vim/blob/master/LICENSE)
* [Credits](#credits)

## Motivation

Bringing emails to the terminal is a pain. The mainstream TUI, (neo)mutt, takes
time to configure. The default mapping is not intuitive when coming from the
Vim environment. It is even scary to use at the beginning, since you are
dealing with sensitive data!

The aim of Himalaya is to extract the email logic into a simple (yet solid) CLI
API that can be used either directly from the terminal or UIs. It gives users
more flexibility.

This Vim plugin is a TUI implementation for Himalaya CLI.

## Installation

First you need to install and configure the [himalaya
CLI](https://github.com/soywod/himalaya#installation). Then you can install
this plugin with your favorite plugin manager. For example with
[vim-plug](https://github.com/junegunn/vim-plug), add to your `.vimrc`:

```viml
Plug 'soywod/himalaya', {'rtp': 'vim'}
```

Then:

```viml
:PlugInstall
```

## Usage

### List messages view

```vim
:Himalaya
```

![gif](https://user-images.githubusercontent.com/10437171/110707014-f9ef1580-81f8-11eb-93ad-233010733ca3.gif)

| Function | Default binding |
| --- | --- |
| Change the current mbox | `gm` |
| Show previous page | `gp` |
| Show next page | `gn` |
| Read focused msg | `<Enter>` |
| Write a new msg | `gw` |
| Reply to the focused msg | `gr` |
| Reply all to the focused msg | `gR` |
| Forward the focused message | `gf` |
| Download all focused msg attachments | `ga` |

They can be customized:

```vim
nmap gm   <plug>(himalaya-mbox-input)
nmap gp   <plug>(himalaya-mbox-prev-page)
nmap gn   <plug>(himalaya-mbox-next-page)
nmap <cr> <plug>(himalaya-msg-read)
nmap gw   <plug>(himalaya-msg-write)
nmap gr   <plug>(himalaya-msg-reply)
nmap gR   <plug>(himalaya-msg-reply-all)
nmap gf   <plug>(himalaya-msg-forward)
nmap ga   <plug>(himalaya-msg-attachments)
```

### List mailboxes

Default behaviour (basic prompt):

![screenshot](https://user-images.githubusercontent.com/10437171/113631817-51eb3180-966a-11eb-8b13-cd1f1f2539ab.jpeg)

With [telescope](https://github.com/nvim-telescope/telescope.nvim) support:

![screenshot](https://user-images.githubusercontent.com/10437171/113631294-86122280-9669-11eb-8074-1c43c36b65a9.jpeg)

With [fzf](https://github.com/junegunn/fzf) support:

![screenshot](https://user-images.githubusercontent.com/10437171/113631382-acd05900-9669-11eb-817d-c28fd5d9574c.jpeg)

### Read message view

![gif](https://user-images.githubusercontent.com/10437171/110708073-7b937300-81fa-11eb-9f4c-5472cea22e21.gif)

| Function | Default binding |
| --- | --- |
| Write a new msg | `gw` |
| Reply to the msg | `gr` |
| Reply all to the msg | `gR` |
| Forward the message | `gf` |
| Download all msg attachments | `ga` |

They can be customized:

```vim
nmap gw <plug>(himalaya-msg-write)
nmap gr <plug>(himalaya-msg-reply)
nmap gR <plug>(himalaya-msg-reply-all)
nmap gf <plug>(himalaya-msg-forward)
nmap ga <plug>(himalaya-msg-attachments)
```

### Write message view

![gif](https://user-images.githubusercontent.com/10437171/110708795-84387900-81fb-11eb-8f8a-f7e7862e816d.gif)

When you exit this special buffer, you will be prompted 4 choices:

- `Send`: sends the message
- `Draft`: saves the message into the `Drafts` mailbox
- `Quit`: quits the buffer without saving
- `Cancel`: goes back to the message edition

## Credits

- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [isync](https://isync.sourceforge.io/), an email synchronizer for offline usage
- [NeoMutt](https://neomutt.org/), an email terminal user interface
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other email terminal user interface
- [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool over NeoMutt and isync
- [rust-imap](https://github.com/jonhoo/rust-imap), a rust IMAP lib
