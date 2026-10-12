---
cairn: log
change: raw-input-bytes
landed: 2026-10-12
---

# Raw messages read whole as bytes

Reported in [#786] by @Piesson. `MessageArg` read piped stdin line by line with `map_while(Result::ok)`, so the first line that was not valid UTF-8 ended the message: the rest was dropped and the command reported success, or `Message is empty` when that line was the first. `RawCommandArg` had the same pattern.

`MessageArg::parse` now returns bytes: stdin is read whole with `read_to_end`, a message file with `fs::read`, and `crlf::normalize` works on bytes. A message in a non-UTF-8 charset, which the file branch used to refuse, now reaches the backend intact. `RawCommandArg` reads stdin whole the same way and refuses input that is not valid UTF-8.

Verified with `maildir message save`: a piped message holding `\xff` and a bare LF was stored whole, the byte kept and the LF turned into CRLF; whitespace-only input is still refused as empty.

Capabilities moved: **commands** (message argument kept as bytes; raw command stdin read whole).

[#786]: https://github.com/pimalaya/himalaya/issues/786
