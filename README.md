# 📫 Himalaya [![GitHub release](https://img.shields.io/github/v/release/soywod/himalaya?color=success)](https://github.com/soywod/himalaya/releases/latest) [![Matrix](https://img.shields.io/matrix/pimalaya.himalaya:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya.himalaya:matrix.org)

https://pimalaya.org/himalaya/

CLI to manage your emails, based on the
[pimalaya-email](https://sr.ht/~soywod/pimalaya/) library.

![image](https://user-images.githubusercontent.com/10437171/138774902-7b9de5a3-93eb-44b0-8cfb-6d2e11e3b1aa.png)

*Disclaimer: the project is under active development, do not use in
production before the `v1.0.0`.*

## Features

- [Folder listing]
- [Envelopes listing], [searching] and [sorting]
- [Email composition] based on `$EDITOR`
- Email manipulation ([copy]/[move]/[delete])
- [Multi-accounting]
- [Account listing]
- [Account synchronization] for offline usage
- IMAP, Maildir and Notmuch support
- IMAP IDLE mode for [real-time notifications]
- PGP end-to-end encryption
- [Completions] for various shells
- JSON output
- …

[Folder listing]: https://pimalaya.org/himalaya/cli/usage/folders/list.html
[Envelopes listing]: https://pimalaya.org/himalaya/cli/usage/envelopes/list.html
[searching]: https://pimalaya.org/himalaya/cli/usage/envelopes/search.html
[sorting]: https://pimalaya.org/himalaya/cli/usage/envelopes/sort.html
[Email composition]: https://pimalaya.org/himalaya/cli/usage/emails/write.html
[copy]: https://pimalaya.org/himalaya/cli/usage/emails/copy.html
[move]: https://pimalaya.org/himalaya/cli/usage/emails/move.html
[delete]: https://pimalaya.org/himalaya/cli/usage/emails/delete.html
[Multi-accounting]: https://pimalaya.org/himalaya/cli/configuration.html
[Account listing]: https://pimalaya.org/himalaya/cli/usage/accounts/list.html
[Account synchronization]: https://pimalaya.org/himalaya/cli/usage/accounts/synchronize.html
[real-time notifications]: https://pimalaya.org/himalaya/cli/usage/notifications.html
[Completions]: https://pimalaya.org/himalaya/cli/tips/completion.html

## Installation

<table align="center">
<tr>
<td width="50%">
<a href="https://repology.org/project/himalaya/versions">
<img src="https://repology.org/badge/vertical-allrepos/himalaya.svg" alt="Packaging status" />
</a>
</td>
<td width="50%">

```bash
# Arch Linux (official)
$ pacman -S himalaya

# Arch Linux (from sources)
$ yay -S himalaya-git

# Homebrew
$ brew install himalaya

# Scoop
$ scoop install himalaya

# Cargo
$ cargo install himalaya

# Nix
$ nix-env -i himalaya
```

*See the
[documentation](https://pimalaya.org/himalaya/cli/installation.html)
for other installation methods.*

</td>
</tr>
</table>

## Configuration

Please read the
[documentation](https://pimalaya.org/himalaya/cli/configuration.html).

## Contributing

If you find a **bug** that [does not exist
yet](https://todo.sr.ht/~soywod/pimalaya), please send an email at
[~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you have a **question**, please send an email at
[~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a
patch at
[~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht)
using [git send-email](https://git-scm.com/docs/git-send-email) (see
[this guide](https://git-send-email.io/) on how to configure it).

If you want to **subscribe** to the mailing list, please send an email
at
[~soywod/pimalaya+subscribe@lists.sr.ht](mailto:~soywod/pimalaya+subscribe@lists.sr.ht).

If you want to **unsubscribe** to the mailing list, please send an
email at
[~soywod/pimalaya+unsubscribe@lists.sr.ht](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht).

If you want to **discuss** about the project, feel free to join the
[Matrix](https://matrix.org/) workspace
[#pimalaya.himalaya](https://matrix.to/#/#pimalaya.himalaya:matrix.org) or contact me
directly [@soywod](https://matrix.to/#/@soywod:matrix.org).

## Credits

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the
[nlnet](https://nlnet.nl/project/Himalaya/index.html) foundation that
helped Himalaya to receive financial support from the [NGI
Assure](https://www.ngi.eu/ngi-projects/ngi-assure/) program of the
European Commission in September, 2022.

* [himalaya-lib](https://git.sr.ht/~soywod/himalaya-lib)
* [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
* [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
* [isync](https://isync.sourceforge.io/), an email synchronizer for
  offline usage
* [NeoMutt](https://neomutt.org/), an email terminal user interface
* [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other
  email terminal user interface
* [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool
  over NeoMutt and isync
* [rust-imap](https://github.com/jonhoo/rust-imap), a Rust IMAP
  library
* [lettre](https://github.com/lettre/lettre), a Rust mailer library
* [mailparse](https://github.com/staktrace/mailparse), a Rust MIME
  email parser.

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
