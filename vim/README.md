# Vim plugin

## Installation

First you need to install and configure the [himalaya
CLI](https://github.com/soywod/himalaya#installation). Then you can
install this plugin with your favorite plugin manager. For example
with [vim-plug](https://github.com/junegunn/vim-plug), add to your
`.vimrc`:

```viml
Plug 'soywod/himalaya', {'rtp': 'vim'}
```

Then:

```viml
:PlugInstall
```

It is highly recommanded to have this option on:

```viml
set hidden
```

## Configuration

### Mailbox picker provider

```vim
let g:himalaya_mailbox_picker = 'native' | 'fzf' | 'telescope'
```

Defines the provider used for picking mailboxes (default keybind:
`gm`):

- `native`: a vim native input
- `fzf`: https://github.com/junegunn/fzf.vim
- `telescope`: https://github.com/nvim-telescope/telescope.nvim

If no value given, the first loaded (and available) provider will be
used (fzf > telescope > native).

### Telescope preview

```vim
let g:himalaya_telescope_preview_enabled = 0
```

Should enable telescope preview when picking a mailbox with the telescope
provider.

### Contact completion

```vim
let g:himalaya_complete_contact_cmd = '<your completion command>'
```

Define the command to use for contact completion. When this is set,
`completefunc` will be set when composing messages so that contacts can be
completed with `<C-x><C-u>`.

The command must print each possible result on its own line. Each line must
contain tab-separated fields; the first must be the email address, and the
second, if present, must be the name. `%s` in the command will be replaced
with the search query.

For example, to complete contacts with khard, you could use
`khard email --remove-first-line --parsable '%s'` as the completion command.

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
| Download attachments from focused message | `ga` |
| Copy the focused message | `gC` |
| Move the focused message | `gM` |
| Delete the focused message(s) | `gD` |

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
nmap gC   <plug>(himalaya-msg-copy)
nmap gM   <plug>(himalaya-msg-move)
nmap gD   <plug>(himalaya-msg-delete)
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
| Copy the message | `gC` |
| Move the message | `gM` |
| Delete the message | `gD` |

They can be customized:

```vim
nmap gw <plug>(himalaya-msg-write)
nmap gr <plug>(himalaya-msg-reply)
nmap gR <plug>(himalaya-msg-reply-all)
nmap gf <plug>(himalaya-msg-forward)
nmap ga <plug>(himalaya-msg-attachments)
nmap gC <plug>(himalaya-msg-copy)
nmap gM <plug>(himalaya-msg-move)
nmap gD <plug>(himalaya-msg-delete)
```

### Write message view

![gif](https://user-images.githubusercontent.com/10437171/110708795-84387900-81fb-11eb-8f8a-f7e7862e816d.gif)

| Function | Default binding |
| --- | --- |
| Add attachment | `ga` |

They can be customized:

```vim
nmap ga <plug>(himalaya-msg-add-attachment)
```

When you exit this special buffer, you will be prompted 4 choices:

- `Send`: sends the message
- `Draft`: saves the message into the `Drafts` mailbox
- `Quit`: quits the buffer without saving
- `Cancel`: goes back to the message edition
