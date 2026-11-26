---
cairn: delta
change: a-prefix-or-phrase-is-never-dropped
---

# Delta

## MODIFIED Requirements

### Requirement: Per-backend translation (search)
The lowered tree SHALL carry on each text and address leaf whether its text is a prefix (a trailing `*`) or a phrase (quoted), and no translation SHALL answer either as a plain word. IMAP and JMAP SHALL send a prefix as its stem and a phrase as one string, the server matching them as it matches text (owner ruling of 2026-11-26 on the open point). Gmail SHALL quote a phrase whole and refuse a prefix with `search-unsupported` before any request, Gmail having no prefix search. Graph SHALL send a phrase as one quoted KQL string and a prefix as its stem quoted with a trailing `*` (`subject:\"repo*\"`).

#### Scenario: A prefix on Gmail
- WHEN `himalaya --json envelope search subject:repo*` runs on a Gmail account
- THEN it exits 1 coded `search-unsupported` before any request, the message quoting `subject:repo*`

#### Scenario: A prefix and a phrase on Graph
- GIVEN a Graph mailbox holding `weekly report` and `beta weekly report`
- WHEN `envelope search subject:repo*` runs, then `envelope search 'subject:"report weekly"'`
- THEN the first lists both messages, and the second neither

### Requirement: What a backend searches is declared (search)
The text fields SHALL say how a backend matches a prefix and a phrase: the matcher and Graph serve both (`full`), Gmail refuses a prefix (`partial`), IMAP and JMAP send a prefix as its stem and a phrase as one string (`partial`).

### Requirement: A field a backend does not answer is refused with a code (search)
Gmail SHALL refuse a prefix, beside `is:answered` and the keywords but `$Important` and `$Junk`.

### Requirement: The query's fields are listed on -h (search)
The command's long help SHALL say how each backend matches text, a prefix and a phrase.
